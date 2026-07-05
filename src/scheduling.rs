use std::{collections::HashMap, time::Duration};

use crate::db::{mongo_client::MongoClient, user::UsersRepository};

pub(crate) async fn recurring_tasks(client: MongoClient, interval: Duration) {
    let mut ticker = tokio::time::interval(interval);
    let user_repo = UsersRepository::new(client.clone());
    ticker.tick().await; // skip the immediate first tick

    loop {
        ticker.tick().await;

        let mut users = match user_repo.list_db_users().await {
            Ok(users) => users,
            Err(err) => {
                log::error!("Failed to list users in recurring task: {err}");
                continue;
            }
        };

        // incoming_amounts and resource_incoming_amounts required for hourly()
        let mut incoming_amounts: HashMap<String, Vec<f32>> = HashMap::new();
        let mut resource_incoming_amounts: HashMap<(String, String), Vec<f32>> = HashMap::new();

        // first evaluate()
        for user in users.iter_mut() {
            if user.is_unit() {
                continue;
            }

            log::debug!("Recurring task tick for user {}", user.id());

            let evaluation = user.credit_mut().evaluate();
            for booked_subscription in evaluation.booked_subscriptions() {
                incoming_amounts
                    .entry(booked_subscription.receiver().to_string())
                    .or_default()
                    .push(booked_subscription.amount());
            }

            // for bloc/zone users resources are evaluated as well
            if let Some(resources) = user.resources_mut() {
                for (resource_name, resource_credit) in resources.iter_mut() {
                    let resource_eval = resource_credit.evaluate();

                    for booked_sub in resource_eval.booked_subscriptions() {
                        resource_incoming_amounts
                            .entry((booked_sub.receiver().to_string(), resource_name.clone()))
                            .or_default()
                            .push(booked_sub.amount());
                    }
                }
            }
        }

        // second hourly()
        for user in users.iter_mut() {
            let user_id_str = user.id().to_string();

            let incoming = incoming_amounts.remove(&user_id_str).unwrap_or_default();
            if !incoming.is_empty() {
                user.credit_mut().apply_amount(incoming.iter().sum());
            }
            if !user.is_unit() {
                user.credit_mut().hourly(incoming);
            }

            if let Some(resources) = user.resources_mut() {
                for (resource_name, resource_credit) in resources.iter_mut() {
                    let key = (user_id_str.clone(), resource_name.clone());
                    let res_incoming = resource_incoming_amounts.remove(&key).unwrap_or_default();
                    if !res_incoming.is_empty() {
                        resource_credit.apply_amount(res_incoming.iter().sum());
                    }
                    resource_credit.hourly(res_incoming);
                }
            }

            if let Err(err) = user_repo.replace_db_user(&user).await {
                log::error!("Failed to persist user {}: {err}", user.id());
            }
        }
    }
    
}