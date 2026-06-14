use std::{collections::HashMap, sync::Arc};
use utoipa_actix_web::service_config::ServiceConfig;

use actix_web::{HttpResponse, Responder, delete, get, patch, post,
    web::{self, Path},
};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

use crate::{
    api::{
        credit::{CreateCreditRequest, GetCreditResponse, CreditBalanceResponse, ListCreditsResponse},
        subscription::{CreateSubscriptionRequest, GetSubscriptionResponse},
        user::{CreateUserRequest, PatchUserRequest, User, UserType},
    },
    db::{
        credit::CreditRepository, mongo_client::MongoClient, subscription::SubscriptionsRepository,
        user::UsersRepository,
    },
    domain::{self, base_user::BaseUser, bloc_user::BlocUser, credit::Credit},
};

#[derive(Deserialize, Debug, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreditBooking {
    credit_type: String,
    receiver: String,
    value: f32,
}

const BLACKOUT_CONTROLLER_URL_ENV: &str = "BLACKOUT_CONTROLLER_URL";
const AI_WO_A_CONTROLLER_URL_ENV: &str = "AI_WO_A_CONTROLLER_URL";

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct HourlyEvaluationResponse {
    evaluated_users: usize,
    booked_subscriptions: usize,
    blackout_notifications: usize,
    sr_overflow_notifications: usize,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct DailyEvaluationResponse {
    updated_users: usize,
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Hello response")
    )
)]
#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[utoipa::path(
    post,
    path = "/echo",
    request_body = String,
    responses(
        (status = 200, description = "Echo response", body = String)
    )
)]
#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}

// ### USERS

#[utoipa::path(
    get,
    path = "/users/{user_id}",
    tag = "users",
    params(
        ("user_id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User found"),
        (status = 404, description = "User not found")
    )
)]
#[get("/users/{user_id}")]
// `GET /api/users/{user_id}`
async fn get_user(client: web::Data<MongoClient>, user_id: Path<String>) -> impl Responder {
    let user_id = user_id.into_inner();
    let repository = UsersRepository::new(client.get_ref().clone());

    match repository.get_user(&user_id).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => HttpResponse::InternalServerError().body(format!("Failed to get user: {err}")),
    }
}

#[utoipa::path(
    get,
    path = "/users",
    tag = "users",
    responses(
        (status = 200, description = "List of users")
    )
)]
#[get("/users")]
// `GET /api/users`
async fn get_users(client: web::Data<MongoClient>) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());

    match repository.list_users().await {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to list users: {err}"))
        }
    }
}

#[utoipa::path(
    post,
    path = "/users",
    tag = "users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created"),
        (status = 409, description = "User already exists")
    )
)]
#[post("/users")]
// `POST /api/users`
async fn create_user(
    client: web::Data<MongoClient>,
    body: web::Json<CreateUserRequest>,
) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());
    let body = body.into_inner();
    let user_id = body.id();
    let credit = Arc::new(Credit::new(0.0, 0.0, vec![], vec![]));
    let resources: HashMap<String, Credit> = HashMap::new();

    let user = match body.user_type() {
        UserType::Bloc => User::Bloc(BaseUser::<BlocUser>::new(resources, user_id, credit)),
        UserType::Zone => User::Zone(
            domain::base_user::BaseUser::<domain::zone_user::ZoneUser>::new(
                resources, user_id, credit,
            ),
        ),
        UserType::Individual => User::Individual(domain::base_user::BaseUser::<
            domain::individual_user::IndividualUser,
        >::new(user_id, credit)),
        UserType::Unit => User::Unit(
            domain::base_user::BaseUser::<domain::unit_user::UnitUser>::new(user_id, credit),
        ),
    };

    match repository.insert_user(user).await {
        Ok(user) => HttpResponse::Created().json(user),
        Err(crate::db::error::Error::Validation(message)) => {
            HttpResponse::Conflict().body(message)
        }
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to create user: {err}"))
        }
    }
}

#[utoipa::path(
    patch,
    path = "/users/{user_id}",
    tag = "users",
    params(
        ("user_id" = String, Path, description = "User ID")
    ),
    request_body = PatchUserRequest,
    responses(
        (status = 200, description = "User updated"),
        (status = 404, description = "User not found"),
        (status = 405, description = "Only unit users can be updated")
    )
)]
#[patch("/users/{user_id}")]
// `PATCH /api/users/{user_id}`
async fn update_user(
    client: web::Data<MongoClient>,
    user_id: Path<String>,
    body: web::Json<PatchUserRequest>,
) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());
    let user_id = user_id.into_inner();
    let body = body.into_inner();

    let user = match repository.get_db_user(&user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().finish(),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Failed to get user: {err}"));
        }
    };

    if !user.is_unit() {
        return HttpResponse::MethodNotAllowed().body("Only unit users can be updated");
    }

    if body.credit_type() != "money" {
        return HttpResponse::NotFound().body("Credit type not found for unit user");
    }

    match repository
        .update_unit_last_day_average(&user_id, body.last_day_average())
        .await
    {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to update user: {err}"))
        }
    }
}

#[utoipa::path(
    post,
    path = "/users/{user_id}/bookings",
    tag = "users",
    params(
        ("user_id" = String, Path, description = "Sender user ID")
    ),
    request_body = CreditBooking,
    responses(
        (status = 200, description = "Booking created"),
        (status = 404, description = "User not found")
    )
)]
#[post("/users/{user_id}/bookings")]
async fn create_booking(
    client: web::Data<MongoClient>,
    user_id: Path<String>,
    body: web::Json<CreditBooking>,
) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());
    let sender_id = user_id.into_inner();
    let body = body.into_inner();

    if body.credit_type == "money" {
        let booking = match repository
            .book_money(&sender_id, &body.receiver, body.value)
            .await
        {
            Ok(booking) => booking,
            Err(crate::db::error::Error::NotFound(_)) => return HttpResponse::NotFound().finish(),
            Err(crate::db::error::Error::Validation(message)) => {
                return HttpResponse::BadRequest().body(message);
            }
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Failed to create booking: {err}"));
            }
        };

        if booking.sender_reached_zero {
            if let Err(err) = notify_blackout_controller(&sender_id).await {
                return HttpResponse::InternalServerError()
                    .body(format!("Failed to notify blackout controller: {err}"));
            }
        }
    } else {
        match repository
            .book_resource(&sender_id, &body.receiver, &body.credit_type, body.value)
            .await
        {
            Ok(()) => {}
            Err(crate::db::error::Error::NotFound(_)) => return HttpResponse::NotFound().finish(),
            Err(crate::db::error::Error::Validation(message)) => {
                return HttpResponse::BadRequest().body(message);
            }
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Failed to create resource booking: {err}"));
            }
        }
    }

    HttpResponse::Ok().finish()
}

// ### CREDITS

#[utoipa::path(
    get,
    path = "/credits/{credit_id}",
    tag = "credits",
    params(
        ("credit_id" = String, Path, description = "Credit ID")
    ),
    responses(
        (status = 200, description = "Credit found", body = GetCreditResponse),
        (status = 400, description = "Invalid credit ID format"),
        (status = 404, description = "Credit not found")
    )
)]
#[get("/credits/{credit_id}")]
// `GET /api/credits/{credit_id}`
async fn get_credit(client: web::Data<MongoClient>, credit_id: Path<String>) -> impl Responder {
    let repository = CreditRepository::new(
        client.get_ref().clone(),
        SubscriptionsRepository::new(client.get_ref().clone()),
    );
    let credit_id = credit_id.into_inner();

    match repository.get_credit(&credit_id).await {
        Ok(Some(credit)) => {
            HttpResponse::Ok().json(GetCreditResponse::from_credit(credit_id, credit))
        }
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(crate::db::error::Error::DbObjectIdError(_)) => {
            HttpResponse::BadRequest().body("Invalid credit id format")
        }
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to get credit: {err}"))
        }
    }
}

#[utoipa::path(
    post,
    path = "/credits",
    tag = "credits",
    request_body = CreateCreditRequest,
    responses(
        (status = 201, description = "Credit created", body = GetCreditResponse),
        (status = 400, description = "Invalid subscription ID format")
    )
)]
#[post("/credits")]
// `POST /api/credits`
async fn create_credit(
    client: web::Data<MongoClient>,
    body: web::Json<CreateCreditRequest>,
) -> impl Responder {
    let body = body.into_inner();
    let subscription_repository = SubscriptionsRepository::new(client.get_ref().clone());
    let credit_repository = CreditRepository::new(
        client.get_ref().clone(),
        SubscriptionsRepository::new(client.get_ref().clone()),
    );

    let mut domain_subscriptions = Vec::with_capacity(body.subscription_ids().len());
    for subscription_id in body.subscription_ids() {
        let object_id = match mongodb::bson::oid::ObjectId::parse_str(subscription_id) {
            Ok(object_id) => object_id,
            Err(_) => return HttpResponse::BadRequest().body("Invalid subscription id format"),
        };

        let subscription = match subscription_repository.get_subscription(&object_id).await {
            Ok(Some(subscription)) => subscription,
            Ok(None) => {
                return HttpResponse::NotFound().body("One or more subscriptions were not found");
            }
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Failed to resolve subscriptions: {err}"));
            }
        };

        domain_subscriptions.push(Arc::new(subscription));
    }

    let domain_credit = domain::credit::Credit::new(
        body.total(),
        body.last_day_average(),
        domain_subscriptions,
        body.history().to_vec(),
    );

    match credit_repository.insert_credit(domain_credit).await {
        Ok(id) => HttpResponse::Created().json(GetCreditResponse::from_create_request(id, &body)),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to create credit: {err}"))
        }
    }
}

// ### SUBSCRIPTIONS

#[utoipa::path(
    get,
    path = "/subscriptions/{subscription_id}",
    tag = "subscriptions",
    params(
        ("subscription_id" = String, Path, description = "Subscription ID")
    ),
    responses(
        (status = 200, description = "Subscription found", body = GetSubscriptionResponse),
        (status = 400, description = "Invalid subscription ID format"),
        (status = 404, description = "Subscription not found")
    )
)]
#[get("/subscriptions/{subscription_id}")]
// `GET /api/subscriptions/{subscription_id}`
async fn get_subscription(
    client: web::Data<MongoClient>,
    subscription_id: Path<String>,
) -> impl Responder {
    let repository = SubscriptionsRepository::new(client.get_ref().clone());
    let subscription_id = subscription_id.into_inner();
    let object_id = match mongodb::bson::oid::ObjectId::parse_str(&subscription_id) {
        Ok(object_id) => object_id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid subscription id format"),
    };

    match repository.get_subscription(&object_id).await {
        Ok(Some(subscription)) => {
            HttpResponse::Ok().json(GetSubscriptionResponse::from(subscription))
        }
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to get subscription: {err}"))
        }
    }
}

#[utoipa::path(
    post,
    path = "/users/{user_id}/subscriptions",
    tag = "users",
    params(
        ("user_id" = String, Path, description = "User ID")
    ),
    request_body = CreateSubscriptionRequest,
    responses(
        (status = 201, description = "Subscription created", body = GetSubscriptionResponse),
        (status = 400, description = "Bad request"),
        (status = 404, description = "User or receiver not found")
    )
)]
#[post("/users/{user_id}/subscriptions")]
// `POST /api/users/{user_id}/subscriptions`
async fn create_subscription(
    client: web::Data<MongoClient>,
    user_id: Path<String>,
    body: web::Json<CreateSubscriptionRequest>,
) -> impl Responder {
    let users_repository = UsersRepository::new(client.get_ref().clone());
    let repository = SubscriptionsRepository::new(client.get_ref().clone());
    let user_id = user_id.into_inner();

    match users_repository.get_db_user(&user_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return HttpResponse::NotFound().body("User not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to get user: {err}"));
        }
    }

    let receiver = match users_repository.get_db_user(body.receiver()).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().body("Receiver user not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to get receiver user: {err}"));
        }
    };

    if receiver.is_unit() {
        return HttpResponse::BadRequest().body("Unit users cannot have incoming subscriptions");
    }

    let credit_type = body.credit_type().to_string();
    let domain_subscription = domain::subscription::Subscription::new(
        mongodb::bson::oid::ObjectId::new().to_hex(),
        body.receiver().to_string(),
        body.value(),
        body.subscription_type(),
        body.priority(),
        credit_type.clone(),
    );

    match repository.insert_subscription(domain_subscription.clone()).await {
        Ok(Some(subscription)) => {
            let attach_result = if credit_type == "money" {
                users_repository
                    .add_subscription_to_user_credit(&user_id, subscription.clone())
                    .await
            } else {
                match users_repository
                    .add_subscription_to_user_resource_credit(
                        &user_id,
                        &credit_type,
                        subscription.clone(),
                    )
                    .await
                {
                    Ok(result) => Ok(result),
                    Err(crate::db::error::Error::Validation(msg)) => {
                        return HttpResponse::BadRequest().body(msg);
                    }
                    Err(err) => Err(err),
                }
            };
            match attach_result {
                Ok(Some(_)) => HttpResponse::Created().json(GetSubscriptionResponse::from(subscription)),
                Ok(None) => HttpResponse::NotFound().body("User not found"),
                Err(err) => HttpResponse::InternalServerError()
                    .body(format!("Failed to attach subscription to user: {err}")),
            }
        }
        Ok(None) => {
            HttpResponse::InternalServerError().body("Subscription creation returned no data")
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to create subscription: {err}")),
    }
}

#[utoipa::path(
    post,
    path = "/evaluations/hourly",
    tag = "evaluations",
    responses(
        (status = 200, description = "Hourly evaluation completed", body = HourlyEvaluationResponse)
    )
)]
#[post("/evaluations/hourly")]
async fn evaluate_hourly(client: web::Data<MongoClient>) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());
    let mut users = match repository.list_db_users().await {
        Ok(users) => users,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to list users for evaluation: {err}"));
        }
    };

    let mut incoming_amounts: HashMap<String, Vec<f32>> = HashMap::new();
    // keyed by (receiver_user_id, resource_name)
    let mut resource_incoming_amounts: HashMap<(String, String), Vec<f32>> = HashMap::new();
    let mut blackout_notifications = Vec::new();
    let mut sr_overflow_notifications = Vec::new();
    let mut evaluated_users = 0usize;
    let mut booked_subscriptions = 0usize;

    for user in users.iter_mut() {
        if user.is_unit() {
            continue;
        }

        evaluated_users += 1;
        let user_id = user.id().to_string();

        // Evaluate money credit
        let evaluation = user.credit_mut().evaluate();
        booked_subscriptions += evaluation.booked_subscriptions().len();

        for booked_subscription in evaluation.booked_subscriptions() {
            incoming_amounts
                .entry(booked_subscription.receiver().to_string())
                .or_default()
                .push(booked_subscription.amount());
        }

        if user.is_individual() && evaluation.hit_zero() {
            blackout_notifications.push(user_id.clone());
        }

        for overflow in evaluation.sr_overflows() {
            sr_overflow_notifications.push((user_id.clone(), *overflow));
        }

        // Evaluate resource credits (Bloc/Zone only)
        if let Some(resources) = user.resources_mut() {
            for (resource_name, resource_credit) in resources.iter_mut() {
                let resource_eval = resource_credit.evaluate();
                booked_subscriptions += resource_eval.booked_subscriptions().len();

                for booked_sub in resource_eval.booked_subscriptions() {
                    resource_incoming_amounts
                        .entry((booked_sub.receiver().to_string(), resource_name.clone()))
                        .or_default()
                        .push(booked_sub.amount());
                }

                for overflow in resource_eval.sr_overflows() {
                    sr_overflow_notifications.push((user_id.clone(), *overflow));
                }
            }
        }
    }

    for user in users.iter_mut() {
        let user_id_str = user.id().to_string();

        // Apply money credit incoming and record hourly history
        let incoming = incoming_amounts.remove(&user_id_str).unwrap_or_default();
        if !incoming.is_empty() {
            let incoming_sum: f32 = incoming.iter().sum();
            user.credit_mut().apply_amount(incoming_sum);
        }
        if !user.is_unit() {
            user.credit_mut().hourly(incoming);
        }

        // Apply resource credit incoming and record hourly history (Bloc/Zone only)
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

        if let Err(err) = repository.replace_db_user(user).await {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to persist evaluated user: {err}"));
        }
    }

    for user_id in &blackout_notifications {
        if let Err(err) = notify_blackout_controller(user_id).await {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to notify blackout controller: {err}"));
        }
    }

    for (user_id, overflow) in &sr_overflow_notifications {
        if let Err(err) = notify_ai_wo_a_controller(user_id, *overflow).await {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to notify AI-WO-A controller: {err}"));
        }
    }

    HttpResponse::Ok().json(HourlyEvaluationResponse {
        evaluated_users,
        booked_subscriptions,
        blackout_notifications: blackout_notifications.len(),
        sr_overflow_notifications: sr_overflow_notifications.len(),
    })
}

#[utoipa::path(
    post,
    path = "/evaluations/daily",
    tag = "evaluations",
    responses(
        (status = 200, description = "Daily evaluation completed", body = DailyEvaluationResponse)
    )
)]
#[post("/evaluations/daily")]
async fn evaluate_daily(client: web::Data<MongoClient>) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());
    let mut users = match repository.list_db_users().await {
        Ok(users) => users,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to list users for daily evaluation: {err}"));
        }
    };

    let mut updated_users = 0usize;

    for user in users.iter_mut() {
        if user.is_unit() {
            continue;
        }

        user.credit_mut().calc_avrg();

        // Also recalculate resource credit averages (Bloc/Zone only)
        if let Some(resources) = user.resources_mut() {
            for resource_credit in resources.values_mut() {
                resource_credit.calc_avrg();
            }
        }

        updated_users += 1;

        if let Err(err) = repository.replace_db_user(user).await {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to persist daily user evaluation: {err}"));
        }
    }

    HttpResponse::Ok().json(DailyEvaluationResponse { updated_users })
}

async fn notify_blackout_controller(user_id: &str) -> Result<(), reqwest::Error> {
    let base_url = std::env::var(BLACKOUT_CONTROLLER_URL_ENV)
        .unwrap_or_else(|_| "http://BLACKOUT-SERVICE".to_string());
    let url = format!(
        "{}/api/credit-overflow?id={user_id}",
        base_url.trim_end_matches('/')
    );

    reqwest::Client::new()
        .get(url)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

async fn notify_ai_wo_a_controller(user_id: &str, overflow: f32) -> Result<(), reqwest::Error> {
    let base_url = std::env::var(AI_WO_A_CONTROLLER_URL_ENV)
        .unwrap_or_else(|_| "http://AI-WO-A-SERVICE".to_string());
    let url = format!(
        "{}/api/credit-overflow?id={user_id}&overflow={overflow}",
        base_url.trim_end_matches('/')
    );

    reqwest::Client::new()
        .get(url)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct ListSubscriptionsResponse {
    subscriptions: Vec<GetSubscriptionResponse>,
}

#[utoipa::path(
    get,
    path = "/users/{user_id}/subscriptions",
    tag = "users",
    params(
        ("user_id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User subscriptions", body = ListSubscriptionsResponse),
        (status = 404, description = "User not found")
    )
)]
#[get("/users/{user_id}/subscriptions")]
// `GET /api/users/{user_id}/subscriptions`
async fn list_user_subscriptions(
    client: web::Data<MongoClient>,
    user_id: Path<String>,
) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());
    let user_id = user_id.into_inner();

    match repository.list_user_subscriptions(&user_id).await {
        Ok(Some(subs)) => HttpResponse::Ok().json(ListSubscriptionsResponse {
            subscriptions: subs.into_iter().map(GetSubscriptionResponse::from).collect(),
        }),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to list subscriptions: {err}")),
    }
}

#[utoipa::path(
    get,
    path = "/users/{user_id}/subscriptions/{subscription_id}",
    tag = "users",
    params(
        ("user_id" = String, Path, description = "User ID"),
        ("subscription_id" = String, Path, description = "Subscription ID")
    ),
    responses(
        (status = 200, description = "Subscription found", body = GetSubscriptionResponse),
        (status = 404, description = "User or subscription not found")
    )
)]
#[get("/users/{user_id}/subscriptions/{subscription_id}")]
// `GET /api/users/{user_id}/subscriptions/{subscription_id}`
async fn get_user_subscription(
    client: web::Data<MongoClient>,
    path: Path<(String, String)>,
) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());
    let (user_id, subscription_id) = path.into_inner();

    match repository.list_user_subscriptions(&user_id).await {
        Ok(Some(subs)) => {
            match subs.into_iter().find(|s| s.id() == subscription_id) {
                Some(sub) => HttpResponse::Ok().json(GetSubscriptionResponse::from(sub)),
                None => HttpResponse::NotFound().finish(),
            }
        }
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to get subscription: {err}")),
    }
}

#[utoipa::path(
    delete,
    path = "/users/{user_id}/subscriptions/{subscription_id}",
    tag = "users",
    params(
        ("user_id" = String, Path, description = "User ID"),
        ("subscription_id" = String, Path, description = "Subscription ID")
    ),
    responses(
        (status = 200, description = "Subscription deleted"),
        (status = 404, description = "User not found")
    )
)]
#[delete("/users/{user_id}/subscriptions/{subscription_id}")]
// `DELETE /api/users/{user_id}/subscriptions/{subscription_id}`
async fn delete_user_subscription(
    client: web::Data<MongoClient>,
    path: Path<(String, String)>,
) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());
    let (user_id, subscription_id) = path.into_inner();

    match repository
        .remove_user_subscription(&user_id, &subscription_id)
        .await
    {
        Ok(Some(_)) => HttpResponse::Ok().finish(),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to delete subscription: {err}")),
    }
}

#[utoipa::path(
    get,
    path = "/users/{user_id}/credits",
    tag = "users",
    params(
        ("user_id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User credits", body = ListCreditsResponse),
        (status = 404, description = "User not found")
    )
)]
#[get("/users/{user_id}/credits")]
// `GET /api/users/{user_id}/credits`
async fn get_user_credits(
    client: web::Data<MongoClient>,
    user_id: Path<String>,
) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());
    let user_id = user_id.into_inner();

    match repository.get_db_user(&user_id).await {
        Ok(Some(user)) => {
            let mut credits = vec![
                CreditBalanceResponse::from_credit("money".to_string(), &user.credit()),
            ];

            if let Some(resources) = user.resources() {
                for (name, credit) in resources.iter() {
                    credits.push(CreditBalanceResponse::from_credit(name.clone(), credit));
                }
            }

            HttpResponse::Ok().json(ListCreditsResponse { credits })
        }
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to get user credits: {err}")),
    }
}

#[utoipa::path(
    get,
    path = "/users/{user_id}/credits/{credit_type}",
    tag = "users",
    params(
        ("user_id" = String, Path, description = "User ID"),
        ("credit_type" = String, Path, description = "Credit type (money or resource name)")
    ),
    responses(
        (status = 200, description = "User credit", body = GetCreditResponse),
        (status = 404, description = "User or credit not found")
    )
)]
#[get("/users/{user_id}/credits/{credit_type}")]
// `GET /api/users/{user_id}/credits/{credit_type}`
async fn get_user_credit_by_type(
    client: web::Data<MongoClient>,
    path: Path<(String, String)>,
) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());
    let (user_id, credit_type) = path.into_inner();

    match repository.get_db_user(&user_id).await {
        Ok(Some(user)) => {
            let credit = if credit_type == "money" {
                Some(user.credit().clone())
            } else if let Some(resources) = user.resources() {
                resources.get(&credit_type).cloned()
            } else {
                None
            };

            match credit {
                Some(c) => HttpResponse::Ok()
                    .json(CreditBalanceResponse::from_credit(credit_type, &c)),
                None => HttpResponse::NotFound().finish(),
            }
        }
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to get user credit: {err}")),
    }
}

pub(crate) fn configure_routes(cfg: &mut ServiceConfig<'_>) {
    cfg.service(hello)
        .service(echo)
        .service(get_user)
        .service(get_users)
        .service(create_user)
        .service(create_booking)
        .service(update_user)
        .service(evaluate_hourly)
        .service(evaluate_daily)
        .service(get_credit)
        .service(create_credit)
        .service(get_user_credits)
        .service(get_user_credit_by_type)
        .service(get_subscription)
        .service(list_user_subscriptions)
        .service(get_user_subscription)
        .service(delete_user_subscription)
        .service(create_subscription);
}

#[derive(OpenApi)]
#[openapi(
    paths(
        hello,
        echo,
        get_user,
        get_users,
        create_user,
        create_booking,
        update_user,
        evaluate_hourly,
        evaluate_daily,
        get_credit,
        create_credit,
        get_user_credits,
        get_user_credit_by_type,
        get_subscription,
        list_user_subscriptions,
        get_user_subscription,
        delete_user_subscription,
        create_subscription,
    ),
    components(
        schemas(
            CreateUserRequest,
            PatchUserRequest,
            UserType,
            GetCreditResponse,
            CreateCreditRequest,
            ListCreditsResponse,
            GetSubscriptionResponse,
            CreateSubscriptionRequest,
            CreditBooking,
            HourlyEvaluationResponse,
            DailyEvaluationResponse,
            ListSubscriptionsResponse,
        )
    ),
    info(
        title = "Credit Exchanger API",
        version = "1.0.0",
        description = "Credit and subscription management system"
    )
)]
pub struct ApiDoc;


#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test};
    use mockito::Server;
    use serial_test::serial;
    use std::env;

    const TEST_DB_URI_ENV: &str = "TEST_MONGODB_URI";
    const TEST_DB_PREFIX: &str = "credit_exchanger_test";

    fn test_db_uri() -> String {
        env::var(TEST_DB_URI_ENV).unwrap_or_else(|_| "mongodb://localhost:27017".to_string())
    }

    async fn test_client() -> web::Data<crate::db::mongo_client::MongoClient> {
        let db_name = format!(
            "{}_{}",
            TEST_DB_PREFIX,
            mongodb::bson::oid::ObjectId::new().to_hex()
        );
        let client = crate::db::mongo_client::MongoClient::new(&test_db_uri(), &db_name)
            .await
            .expect("Failed to connect to test MongoDB");

        web::Data::new(client)
    }

    // ── Users ─────────────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_get_user_returns_user() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(get_user),
        )
        .await;

        let create_req_1 = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "alice", "userType": "individual"}))
            .to_request();
        test::call_service(&app, create_req_1).await;

        let create_req_2 = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "bob", "userType": "unit"}))
            .to_request();
        test::call_service(&app, create_req_2).await;

        let get_req = test::TestRequest::get().uri("/users/alice").to_request();
        let resp = test::call_service(&app, get_req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["id"], "alice");
        assert_eq!(body["userType"], "individual")
    }

    #[actix_web::test]
    async fn test_get_user_not_found() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(get_user)).await;
        let req = test::TestRequest::get()
            .uri("/users/no_such_user_xyz")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_get_users_returns_empty() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(get_users)).await;
        let req = test::TestRequest::get().uri("/users").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body, serde_json::json!([]));
    }

    #[actix_web::test]
    async fn test_get_users_returns_all() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(get_users),
        )
        .await;

        let bloc_id = format!("bloc_{}", mongodb::bson::oid::ObjectId::new().to_hex());
        let zone_id = format!("zone_{}", mongodb::bson::oid::ObjectId::new().to_hex());
        let individual_id = format!(
            "individual_{}",
            mongodb::bson::oid::ObjectId::new().to_hex()
        );
        let unit_id = format!("unit_{}", mongodb::bson::oid::ObjectId::new().to_hex());

        for (id, user_type) in [
            (&bloc_id, "bloc"),
            (&zone_id, "zone"),
            (&individual_id, "individual"),
            (&unit_id, "unit"),
        ] {
            let req = test::TestRequest::post()
                .uri("/users")
                .set_json(serde_json::json!({ "id": id, "userType": user_type }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);
        }

        let req = test::TestRequest::get().uri("/users").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let users: serde_json::Value = test::read_body_json(resp).await;
        let user_array = users.as_array().expect("users response should be an array");

        let returned_ids: std::collections::HashSet<String> = user_array
            .iter()
            .filter_map(|user| {
                user.get("id")
                    .and_then(serde_json::Value::as_str)
                    .map(ToString::to_string)
            })
            .collect();

        assert!(returned_ids.contains(&bloc_id));
        assert!(returned_ids.contains(&zone_id));
        assert!(returned_ids.contains(&individual_id));
        assert!(returned_ids.contains(&unit_id));
    }

    #[actix_web::test]
    async fn test_create_user_bloc() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_user)).await;
        let req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "test_bloc_user", "userType": "bloc"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["id"], "test_bloc_user");
        assert_eq!(body["userType"], "bloc");
        assert_eq!(body["resources"], serde_json::json!({}));
        assert_eq!(body["credit"]["total"], 0.0);
        assert_eq!(body["credit"]["last_day_average"], 0.0);
        assert_eq!(body["credit"]["subscriptions"], serde_json::json!([]));
        assert_eq!(body["credit"]["history"], serde_json::json!([]));
    }

    #[actix_web::test]
    async fn test_create_user_zone() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_user)).await;
        let req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "test_zone_user", "userType": "zone"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["id"], "test_zone_user");
        assert_eq!(body["userType"], "zone");
        assert_eq!(body["resources"], serde_json::json!({}));
        assert_eq!(body["credit"]["total"], 0.0);
        assert_eq!(body["credit"]["last_day_average"], 0.0);
        assert_eq!(body["credit"]["subscriptions"], serde_json::json!([]));
        assert_eq!(body["credit"]["history"], serde_json::json!([]));
    }

    #[actix_web::test]
    async fn test_create_user_individual() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_user)).await;
        let req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "test_individual_user", "userType": "individual"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["id"], "test_individual_user");
        assert_eq!(body["userType"], "individual");
        assert_eq!(body["credit"]["total"], 0.0);
        assert_eq!(body["credit"]["last_day_average"], 0.0);
        assert_eq!(body["credit"]["subscriptions"], serde_json::json!([]));
        assert_eq!(body["credit"]["history"], serde_json::json!([]));
    }

    #[actix_web::test]
    async fn test_create_user_unit() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_user)).await;
        let req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "test_unit_user", "userType": "unit"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["id"], "test_unit_user");
        assert_eq!(body["userType"], "unit");
        assert_eq!(body["credit"]["total"], 0.0);
        assert_eq!(body["credit"]["last_day_average"], 0.0);
        assert_eq!(body["credit"]["subscriptions"], serde_json::json!([]));
        assert_eq!(body["credit"]["history"], serde_json::json!([]));
    }

    #[actix_web::test]
    async fn test_create_user_duplicate_id_returns_conflict() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_user)).await;

        let first_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "duplicate_user", "userType": "individual"}))
            .to_request();
        let first_resp = test::call_service(&app, first_req).await;
        assert_eq!(first_resp.status(), actix_web::http::StatusCode::CREATED);

        let second_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "duplicate_user", "userType": "individual"}))
            .to_request();
        let second_resp = test::call_service(&app, second_req).await;
        assert_eq!(second_resp.status(), actix_web::http::StatusCode::CONFLICT);
    }

    #[actix_web::test]
    async fn test_create_user_unknown_type_returns_bad_request() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_user)).await;
        let req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "bad_user", "userType": "unknownType"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_update_user_not_found() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(update_user)).await;
        let req = test::TestRequest::patch()
            .uri("/users/no_such_user_xyz")
            .set_json(serde_json::json!({"creditType": "money", "lastDayAverage": 10.0}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_update_non_unit_user_returns_method_not_allowed() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(update_user),
        )
        .await;

        // Create a bloc user first
        let create_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "patch_bloc_user", "userType": "bloc"}))
            .to_request();
        test::call_service(&app, create_req).await;

        let patch_req = test::TestRequest::patch()
            .uri("/users/patch_bloc_user")
            .set_json(serde_json::json!({"creditType": "money", "lastDayAverage": 10.0}))
            .to_request();
        let resp = test::call_service(&app, patch_req).await;
        assert_eq!(
            resp.status(),
            actix_web::http::StatusCode::METHOD_NOT_ALLOWED
        );
    }

    #[actix_web::test]
    async fn test_update_unit_user_credit_type_not_money_returns_not_found() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(update_user),
        )
        .await;

        let create_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "patch_unit_user_bad_credit", "userType": "unit"}))
            .to_request();
        test::call_service(&app, create_req).await;

        let patch_req = test::TestRequest::patch()
            .uri("/users/patch_unit_user_bad_credit")
            .set_json(serde_json::json!({"creditType": "unknown", "lastDayAverage": 10.0}))
            .to_request();
        let resp = test::call_service(&app, patch_req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_update_unit_user_returns_ok() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(update_user),
        )
        .await;

        let create_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "patch_unit_user", "userType": "unit"}))
            .to_request();
        test::call_service(&app, create_req).await;

        let patch_req = test::TestRequest::patch()
            .uri("/users/patch_unit_user")
            .set_json(serde_json::json!({"creditType": "money", "lastDayAverage": 42.0}))
            .to_request();
        let resp = test::call_service(&app, patch_req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
    }

    #[actix_web::test]
    #[serial]
    async fn test_create_booking_notifies_blackout_when_individual_hits_zero() {
        let mut blackout_server = Server::new_async().await;
        let blackout_mock = blackout_server
            .mock("GET", "/api/credit-overflow")
            .match_query(mockito::Matcher::UrlEncoded(
                "id".into(),
                "booking_individual_user".into(),
            ))
            .with_status(200)
            .create_async()
            .await;

        unsafe {
            env::set_var(BLACKOUT_CONTROLLER_URL_ENV, blackout_server.url());
        }

        let client = test_client().await;
        let mongo_client = client.get_ref().clone();
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(create_booking)
                .service(get_user),
        )
        .await;

        for (id, user_type) in [
            ("booking_individual_user", "individual"),
            ("booking_receiver_user", "unit"),
        ] {
            let create_req = test::TestRequest::post()
                .uri("/users")
                .set_json(serde_json::json!({"id": id, "userType": user_type}))
                .to_request();
            let create_resp = test::call_service(&app, create_req).await;
            assert_eq!(create_resp.status(), actix_web::http::StatusCode::CREATED);
        }

        mongo_client
            .update_document_by_field(
                "users",
                "id",
                "booking_individual_user",
                mongodb::bson::doc! {
                    "$set": {
                        "credit.total": 10.0,
                    }
                },
            )
            .await
            .expect("failed to seed sender credit");

        let booking_req = test::TestRequest::post()
            .uri("/users/booking_individual_user/bookings")
            .set_json(serde_json::json!({
                "creditType": "money",
                "receiver": "booking_receiver_user",
                "value": 10.0
            }))
            .to_request();
        let booking_resp = test::call_service(&app, booking_req).await;
        assert_eq!(booking_resp.status(), actix_web::http::StatusCode::OK);

        blackout_mock.assert_async().await;

        let get_sender_req = test::TestRequest::get()
            .uri("/users/booking_individual_user")
            .to_request();
        let get_sender_resp = test::call_service(&app, get_sender_req).await;
        assert_eq!(get_sender_resp.status(), actix_web::http::StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(get_sender_resp).await;
        assert_eq!(body["credit"]["total"], 0.0);

        unsafe {
            env::remove_var(BLACKOUT_CONTROLLER_URL_ENV);
        }
    }

    // ── Credits ───────────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_get_credit_invalid_id_returns_bad_request() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(get_credit)).await;
        let req = test::TestRequest::get()
            .uri("/credits/not_an_object_id")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_get_credit_not_found() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(get_credit)).await;
        let req = test::TestRequest::get()
            .uri("/credits/000000000000000000000001")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_create_credit_no_subscriptions() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_credit)).await;
        let req = test::TestRequest::post()
            .uri("/credits")
            .set_json(serde_json::json!({
                "total": 100.0,
                "lastDayAverage": 50.0,
                "subscriptionIds": [],
                "history": [10.0, 20.0]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);
    }

    #[actix_web::test]
    async fn test_create_credit_invalid_subscription_id_returns_bad_request() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_credit)).await;
        let req = test::TestRequest::post()
            .uri("/credits")
            .set_json(serde_json::json!({
                "total": 100.0,
                "lastDayAverage": 50.0,
                "subscriptionIds": ["not_a_valid_object_id"],
                "history": []
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_create_credit_nonexistent_subscription_returns_not_found() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_credit)).await;
        let req = test::TestRequest::post()
            .uri("/credits")
            .set_json(serde_json::json!({
                "total": 100.0,
                "lastDayAverage": 50.0,
                "subscriptionIds": ["000000000000000000000001"],
                "history": []
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    // ── Subscriptions ─────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_get_subscription_invalid_id_returns_bad_request() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(get_subscription)).await;
        let req = test::TestRequest::get()
            .uri("/subscriptions/not_an_object_id")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_get_subscription_not_found() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(get_subscription)).await;
        let req = test::TestRequest::get()
            .uri("/subscriptions/000000000000000000000001")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_create_subscription_returns_created() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(get_user)
                .service(create_subscription),
        )
        .await;

        let create_owner_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "subscription_owner", "userType": "individual"}))
            .to_request();
        let create_owner_resp = test::call_service(&app, create_owner_req).await;
        assert_eq!(create_owner_resp.status(), actix_web::http::StatusCode::CREATED);

        let create_user_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "subscription_receiver", "userType": "individual"}))
            .to_request();
        let create_user_resp = test::call_service(&app, create_user_req).await;
        assert_eq!(create_user_resp.status(), actix_web::http::StatusCode::CREATED);

        let req = test::TestRequest::post()
            .uri("/users/subscription_owner/subscriptions")
            .set_json(serde_json::json!({
                "receiver": "subscription_receiver",
                "value": 10.5,
                "subscriptionType": "sr",
                "priority": 1
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);

        let get_user_req = test::TestRequest::get()
            .uri("/users/subscription_owner")
            .to_request();
        let get_user_resp = test::call_service(&app, get_user_req).await;
        assert_eq!(get_user_resp.status(), actix_web::http::StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(get_user_resp).await;
        assert_eq!(body["credit"]["subscriptions"].as_array().map(Vec::len), Some(1));
    }

    #[actix_web::test]
    async fn test_create_and_get_subscription() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(create_subscription)
                .service(get_subscription),
        )
        .await;

        let create_owner_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "subscription_get_owner", "userType": "individual"}))
            .to_request();
        let create_owner_resp = test::call_service(&app, create_owner_req).await;
        assert_eq!(create_owner_resp.status(), actix_web::http::StatusCode::CREATED);

        let create_user_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "subscription_get_receiver", "userType": "individual"}))
            .to_request();
        let create_user_resp = test::call_service(&app, create_user_req).await;
        assert_eq!(create_user_resp.status(), actix_web::http::StatusCode::CREATED);

        let create_req = test::TestRequest::post()
            .uri("/users/subscription_get_owner/subscriptions")
            .set_json(serde_json::json!({
                "receiver": "subscription_get_receiver",
                "value": 5.0,
                "subscriptionType": "contract",
                "priority": 2
            }))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        assert_eq!(create_resp.status(), actix_web::http::StatusCode::CREATED);

        let body: serde_json::Value = test::read_body_json(create_resp).await;
        let id = body["id"].as_str().expect("id missing in response");

        let get_req = test::TestRequest::get()
            .uri(&format!("/subscriptions/{id}"))
            .to_request();
        let get_resp = test::call_service(&app, get_req).await;
        assert_eq!(get_resp.status(), actix_web::http::StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_create_subscription_for_unit_receiver_returns_bad_request() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(create_subscription),
        )
        .await;

        let create_owner_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "subscription_unit_owner", "userType": "individual"}))
            .to_request();
        let create_owner_resp = test::call_service(&app, create_owner_req).await;
        assert_eq!(create_owner_resp.status(), actix_web::http::StatusCode::CREATED);

        let create_user_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "subscription_unit_receiver", "userType": "unit"}))
            .to_request();
        let create_user_resp = test::call_service(&app, create_user_req).await;
        assert_eq!(create_user_resp.status(), actix_web::http::StatusCode::CREATED);

        let create_subscription_req = test::TestRequest::post()
            .uri("/users/subscription_unit_owner/subscriptions")
            .set_json(serde_json::json!({
                "receiver": "subscription_unit_receiver",
                "value": 12.5,
                "subscriptionType": "sr",
                "priority": 1
            }))
            .to_request();
        let create_subscription_resp = test::call_service(&app, create_subscription_req).await;
        assert_eq!(
            create_subscription_resp.status(),
            actix_web::http::StatusCode::BAD_REQUEST
        );
    }

    #[actix_web::test]
    #[serial]
    async fn test_evaluate_hourly_updates_balances_history_and_notifications() {
        let mut controller_server = Server::new_async().await;
        let blackout_mock = controller_server
            .mock("GET", "/api/credit-overflow")
            .match_query(mockito::Matcher::UrlEncoded(
                "id".into(),
                "hourly_sender_zero".into(),
            ))
            .with_status(200)
            .create_async()
            .await;
        let ai_mock = controller_server
            .mock("GET", "/api/credit-overflow")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("id".into(), "hourly_sender_overflow".into()),
                mockito::Matcher::UrlEncoded("overflow".into(), "5".into()),
            ]))
            .with_status(200)
            .create_async()
            .await;

        unsafe {
            env::set_var(BLACKOUT_CONTROLLER_URL_ENV, controller_server.url());
            env::set_var(AI_WO_A_CONTROLLER_URL_ENV, controller_server.url());
        }

        let client = test_client().await;
        let mongo_client = client.get_ref().clone();
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(get_user)
                .service(create_subscription)
                .service(evaluate_hourly),
        )
        .await;

        for (id, user_type) in [
            ("hourly_sender_zero", "individual"),
            ("hourly_sender_overflow", "individual"),
            ("hourly_receiver", "individual"),
        ] {
            let create_user_req = test::TestRequest::post()
                .uri("/users")
                .set_json(serde_json::json!({"id": id, "userType": user_type}))
                .to_request();
            let create_user_resp = test::call_service(&app, create_user_req).await;
            assert_eq!(create_user_resp.status(), actix_web::http::StatusCode::CREATED);
        }

        for (owner, receiver, sub_type) in [
            ("hourly_sender_zero", "hourly_receiver", "contract"),
            ("hourly_sender_overflow", "hourly_receiver", "sr"),
        ] {
            let create_subscription_req = test::TestRequest::post()
                .uri(&format!("/users/{owner}/subscriptions"))
                .set_json(serde_json::json!({
                    "receiver": receiver,
                    "value": 10.0,
                    "subscriptionType": sub_type,
                    "priority": 1
                }))
                .to_request();
            let create_subscription_resp = test::call_service(&app, create_subscription_req).await;
            assert_eq!(create_subscription_resp.status(), actix_web::http::StatusCode::CREATED);
        }

        for (user_id, total, last_day_average) in [
            ("hourly_sender_zero", 10.0, 100.0),
            ("hourly_sender_overflow", 5.0, 100.0),
            ("hourly_receiver", 0.0, 0.0),
        ] {
            mongo_client
                .update_document_by_field(
                    "users",
                    "id",
                    user_id,
                    mongodb::bson::doc! {
                        "$set": {
                            "credit.total": total,
                            "credit.last_day_average": last_day_average,
                        }
                    },
                )
                .await
                .expect("failed to seed user credit");
        }

        let evaluate_req = test::TestRequest::post()
            .uri("/evaluations/hourly")
            .to_request();
        let evaluate_resp = test::call_service(&app, evaluate_req).await;
        assert_eq!(evaluate_resp.status(), actix_web::http::StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(evaluate_resp).await;
        assert_eq!(body["evaluatedUsers"], 3);
        assert_eq!(body["bookedSubscriptions"], 1);
        assert_eq!(body["blackoutNotifications"], 1);
        assert_eq!(body["srOverflowNotifications"], 1);

        blackout_mock.assert_async().await;
        ai_mock.assert_async().await;

        for (user_id, expected_total, expected_history) in [
            ("hourly_sender_zero", 0.0, serde_json::json!([-10.0])),
            ("hourly_sender_overflow", 5.0, serde_json::json!([-10.0])),
            ("hourly_receiver", 10.0, serde_json::json!([10.0])),
        ] {
            let get_user_req = test::TestRequest::get()
                .uri(&format!("/users/{user_id}"))
                .to_request();
            let get_user_resp = test::call_service(&app, get_user_req).await;
            assert_eq!(get_user_resp.status(), actix_web::http::StatusCode::OK);

            let body: serde_json::Value = test::read_body_json(get_user_resp).await;
            assert_eq!(body["credit"]["total"], expected_total);
            assert_eq!(body["credit"]["history"], expected_history);
        }

        unsafe {
            env::remove_var(BLACKOUT_CONTROLLER_URL_ENV);
            env::remove_var(AI_WO_A_CONTROLLER_URL_ENV);
        }
    }

    #[actix_web::test]
    async fn test_evaluate_daily_updates_non_unit_average_only() {
        let client = test_client().await;
        let mongo_client = client.get_ref().clone();
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(get_user)
                .service(evaluate_daily),
        )
        .await;

        for (id, user_type) in [
            ("daily_individual", "individual"),
            ("daily_unit", "unit"),
        ] {
            let create_user_req = test::TestRequest::post()
                .uri("/users")
                .set_json(serde_json::json!({"id": id, "userType": user_type}))
                .to_request();
            let create_user_resp = test::call_service(&app, create_user_req).await;
            assert_eq!(create_user_resp.status(), actix_web::http::StatusCode::CREATED);
        }

        mongo_client
            .update_document_by_field(
                "users",
                "id",
                "daily_individual",
                mongodb::bson::doc! {
                    "$set": {
                        "credit.history": [10.0, 20.0],
                        "credit.last_day_average": 0.0,
                    }
                },
            )
            .await
            .expect("failed to seed individual history");
        mongo_client
            .update_document_by_field(
                "users",
                "id",
                "daily_unit",
                mongodb::bson::doc! {
                    "$set": {
                        "credit.history": [10.0, 20.0],
                        "credit.last_day_average": 42.0,
                    }
                },
            )
            .await
            .expect("failed to seed unit history");

        let evaluate_req = test::TestRequest::post()
            .uri("/evaluations/daily")
            .to_request();
        let evaluate_resp = test::call_service(&app, evaluate_req).await;
        assert_eq!(evaluate_resp.status(), actix_web::http::StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(evaluate_resp).await;
        assert_eq!(body["updatedUsers"], 1);

        let get_individual_req = test::TestRequest::get()
            .uri("/users/daily_individual")
            .to_request();
        let get_individual_resp = test::call_service(&app, get_individual_req).await;
        let individual_body: serde_json::Value = test::read_body_json(get_individual_resp).await;
        assert_eq!(individual_body["credit"]["last_day_average"], 15.0);
        assert_eq!(individual_body["credit"]["history"], serde_json::json!([]));

        let get_unit_req = test::TestRequest::get().uri("/users/daily_unit").to_request();
        let get_unit_resp = test::call_service(&app, get_unit_req).await;
        let unit_body: serde_json::Value = test::read_body_json(get_unit_resp).await;
        assert_eq!(unit_body["credit"]["last_day_average"], 42.0);
        assert_eq!(unit_body["credit"]["history"], serde_json::json!([10.0, 20.0]));
    }

    // ── Resource credit ───────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_resource_booking_transfers_balance() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(create_booking)
                .service(get_user),
        )
        .await;

        // Create bloc sender with oil resource
        let sender_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "bloc_res_sender", "userType": "bloc"}))
            .to_request();
        test::call_service(&app, sender_req).await;

        // Create bloc receiver
        let receiver_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "bloc_res_receiver", "userType": "bloc"}))
            .to_request();
        test::call_service(&app, receiver_req).await;

        // Seed oil credit via DB directly
        let mongo_client_data = test_client().await;
        // Use a fresh client with the same DB as the app's client by re-using the test app
        // Instead, inject oil resource via resource subscription creation then manually
        // seed balance using the repository
        // Simplest: test via a known-path — add resource by posting a resource subscription first
        // to create the slot, then see booking fails with zero balance.
        // Actually test: booking from sender with 0 oil balance → bad request (insufficient)
        let booking_req = test::TestRequest::post()
            .uri("/users/bloc_res_sender/bookings")
            .set_json(serde_json::json!({
                "creditType": "oil",
                "receiver": "bloc_res_receiver",
                "value": 10.0
            }))
            .to_request();
        let resp = test::call_service(&app, booking_req).await;
        // resource "oil" doesn't exist on sender → NotFound
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_resource_booking_individual_sender_returns_bad_request() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(create_booking),
        )
        .await;

        let sender_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "ind_res_sender", "userType": "individual"}))
            .to_request();
        test::call_service(&app, sender_req).await;

        let receiver_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "bloc_res_recv2", "userType": "bloc"}))
            .to_request();
        test::call_service(&app, receiver_req).await;

        let booking_req = test::TestRequest::post()
            .uri("/users/ind_res_sender/bookings")
            .set_json(serde_json::json!({
                "creditType": "oil",
                "receiver": "bloc_res_recv2",
                "value": 10.0
            }))
            .to_request();
        let resp = test::call_service(&app, booking_req).await;
        // Individual has no resources
        assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_resource_subscription_attached_to_resource_credit() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(create_subscription)
                .service(get_user),
        )
        .await;

        let owner_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "bloc_sub_owner", "userType": "bloc"}))
            .to_request();
        test::call_service(&app, owner_req).await;

        let receiver_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "bloc_sub_receiver", "userType": "bloc"}))
            .to_request();
        test::call_service(&app, receiver_req).await;

        let sub_req = test::TestRequest::post()
            .uri("/users/bloc_sub_owner/subscriptions")
            .set_json(serde_json::json!({
                "receiver": "bloc_sub_receiver",
                "value": 20.0,
                "subscriptionType": "contract",
                "priority": 1,
                "creditType": "oil"
            }))
            .to_request();
        let sub_resp = test::call_service(&app, sub_req).await;
        assert_eq!(sub_resp.status(), actix_web::http::StatusCode::CREATED);

        let sub_body: serde_json::Value = test::read_body_json(sub_resp).await;
        assert_eq!(sub_body["creditType"], "oil");

        // Verify the subscription was added to the owner's oil resource credit
        let get_req = test::TestRequest::get().uri("/users/bloc_sub_owner").to_request();
        let get_resp = test::call_service(&app, get_req).await;
        let user_body: serde_json::Value = test::read_body_json(get_resp).await;
        let oil_subs = &user_body["resources"]["oil"]["subscriptions"];
        assert_eq!(oil_subs.as_array().map(|a| a.len()).unwrap_or(0), 1);
    }

    #[actix_web::test]
    async fn test_resource_subscription_for_individual_owner_returns_bad_request() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(create_subscription),
        )
        .await;

        let owner_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "ind_res_sub_owner", "userType": "individual"}))
            .to_request();
        test::call_service(&app, owner_req).await;

        let receiver_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "bloc_res_sub_recv", "userType": "bloc"}))
            .to_request();
        test::call_service(&app, receiver_req).await;

        let sub_req = test::TestRequest::post()
            .uri("/users/ind_res_sub_owner/subscriptions")
            .set_json(serde_json::json!({
                "receiver": "bloc_res_sub_recv",
                "value": 20.0,
                "subscriptionType": "contract",
                "priority": 1,
                "creditType": "oil"
            }))
            .to_request();
        let sub_resp = test::call_service(&app, sub_req).await;
        assert_eq!(sub_resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    }

    // ── User-scoped subscription routes ───────────────────────────────────────

    #[actix_web::test]
    async fn test_list_user_subscriptions_returns_subscriptions() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(create_subscription)
                .service(list_user_subscriptions),
        )
        .await;

        let owner_id = format!("sub_list_owner_{}", mongodb::bson::oid::ObjectId::new().to_hex());
        let recv_id = format!("sub_list_recv_{}", mongodb::bson::oid::ObjectId::new().to_hex());

        test::call_service(&app, test::TestRequest::post().uri("/users")
            .set_json(serde_json::json!({"id": &owner_id, "userType": "individual"}))
            .to_request()).await;
        test::call_service(&app, test::TestRequest::post().uri("/users")
            .set_json(serde_json::json!({"id": &recv_id, "userType": "individual"}))
            .to_request()).await;

        let sub_req = test::TestRequest::post()
            .uri(&format!("/users/{owner_id}/subscriptions"))
            .set_json(serde_json::json!({
                "receiver": &recv_id, "value": 10.0,
                "subscriptionType": "contract", "priority": 1
            }))
            .to_request();
        test::call_service(&app, sub_req).await;

        let list_req = test::TestRequest::get()
            .uri(&format!("/users/{owner_id}/subscriptions"))
            .to_request();
        let resp = test::call_service(&app, list_req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        let subs = body["subscriptions"].as_array().expect("subscriptions array");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0]["creditType"], "money");
    }

    #[actix_web::test]
    async fn test_list_user_subscriptions_returns_404_for_missing_user() {
        let client = test_client().await;
        let app = test::init_service(
            App::new().app_data(client).service(list_user_subscriptions),
        )
        .await;

        let resp = test::call_service(&app, test::TestRequest::get()
            .uri("/users/no_such_user/subscriptions").to_request()).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_list_user_subscriptions_returns_empty_list() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(list_user_subscriptions),
        )
        .await;

        let id = format!("no_subs_user_{}", mongodb::bson::oid::ObjectId::new().to_hex());
        test::call_service(&app, test::TestRequest::post().uri("/users")
            .set_json(serde_json::json!({"id": &id, "userType": "individual"}))
            .to_request()).await;

        let resp = test::call_service(&app, test::TestRequest::get()
            .uri(&format!("/users/{id}/subscriptions")).to_request()).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["subscriptions"], serde_json::json!([]));
    }

    #[actix_web::test]
    async fn test_get_user_subscription_returns_subscription() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(create_subscription)
                .service(get_user_subscription),
        )
        .await;

        let owner_id = format!("sub_get_owner_{}", mongodb::bson::oid::ObjectId::new().to_hex());
        let recv_id = format!("sub_get_recv_{}", mongodb::bson::oid::ObjectId::new().to_hex());

        test::call_service(&app, test::TestRequest::post().uri("/users")
            .set_json(serde_json::json!({"id": &owner_id, "userType": "individual"}))
            .to_request()).await;
        test::call_service(&app, test::TestRequest::post().uri("/users")
            .set_json(serde_json::json!({"id": &recv_id, "userType": "individual"}))
            .to_request()).await;

        let sub_resp = test::call_service(&app, test::TestRequest::post()
            .uri(&format!("/users/{owner_id}/subscriptions"))
            .set_json(serde_json::json!({
                "receiver": &recv_id, "value": 15.0,
                "subscriptionType": "sr", "priority": 2
            }))
            .to_request()).await;
        let sub_body: serde_json::Value = test::read_body_json(sub_resp).await;
        let sub_id = sub_body["id"].as_str().expect("subscription id");

        let resp = test::call_service(&app, test::TestRequest::get()
            .uri(&format!("/users/{owner_id}/subscriptions/{sub_id}"))
            .to_request()).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["value"], 15.0);
        assert_eq!(body["subscriptionType"], "sr");
    }

    #[actix_web::test]
    async fn test_get_user_subscription_returns_404_for_missing_user() {
        let client = test_client().await;
        let app = test::init_service(
            App::new().app_data(client).service(get_user_subscription),
        )
        .await;

        let resp = test::call_service(&app, test::TestRequest::get()
            .uri("/users/ghost_user/subscriptions/abc123").to_request()).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_get_user_subscription_returns_404_for_missing_subscription() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(get_user_subscription),
        )
        .await;

        let id = format!("sub_miss_user_{}", mongodb::bson::oid::ObjectId::new().to_hex());
        test::call_service(&app, test::TestRequest::post().uri("/users")
            .set_json(serde_json::json!({"id": &id, "userType": "individual"}))
            .to_request()).await;

        let resp = test::call_service(&app, test::TestRequest::get()
            .uri(&format!("/users/{id}/subscriptions/no_such_sub")).to_request()).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_delete_user_subscription_removes_it() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(create_subscription)
                .service(list_user_subscriptions)
                .service(delete_user_subscription),
        )
        .await;

        let owner_id = format!("sub_del_owner_{}", mongodb::bson::oid::ObjectId::new().to_hex());
        let recv_id = format!("sub_del_recv_{}", mongodb::bson::oid::ObjectId::new().to_hex());

        test::call_service(&app, test::TestRequest::post().uri("/users")
            .set_json(serde_json::json!({"id": &owner_id, "userType": "individual"}))
            .to_request()).await;
        test::call_service(&app, test::TestRequest::post().uri("/users")
            .set_json(serde_json::json!({"id": &recv_id, "userType": "individual"}))
            .to_request()).await;

        let sub_resp = test::call_service(&app, test::TestRequest::post()
            .uri(&format!("/users/{owner_id}/subscriptions"))
            .set_json(serde_json::json!({
                "receiver": &recv_id, "value": 5.0,
                "subscriptionType": "contract", "priority": 1
            }))
            .to_request()).await;
        let sub_body: serde_json::Value = test::read_body_json(sub_resp).await;
        let sub_id = sub_body["id"].as_str().expect("subscription id");

        let del_resp = test::call_service(&app, test::TestRequest::delete()
            .uri(&format!("/users/{owner_id}/subscriptions/{sub_id}"))
            .to_request()).await;
        assert_eq!(del_resp.status(), actix_web::http::StatusCode::OK);

        let list_resp = test::call_service(&app, test::TestRequest::get()
            .uri(&format!("/users/{owner_id}/subscriptions"))
            .to_request()).await;
        let list_body: serde_json::Value = test::read_body_json(list_resp).await;
        assert_eq!(list_body["subscriptions"], serde_json::json!([]));
    }

    #[actix_web::test]
    async fn test_delete_user_subscription_returns_404_for_missing_user() {
        let client = test_client().await;
        let app = test::init_service(
            App::new().app_data(client).service(delete_user_subscription),
        )
        .await;

        let resp = test::call_service(&app, test::TestRequest::delete()
            .uri("/users/ghost_user/subscriptions/abc").to_request()).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_delete_user_subscription_succeeds_when_subscription_not_found() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(delete_user_subscription),
        )
        .await;

        let id = format!("sub_del_noop_{}", mongodb::bson::oid::ObjectId::new().to_hex());
        test::call_service(&app, test::TestRequest::post().uri("/users")
            .set_json(serde_json::json!({"id": &id, "userType": "individual"}))
            .to_request()).await;

        let resp = test::call_service(&app, test::TestRequest::delete()
            .uri(&format!("/users/{id}/subscriptions/nonexistent_sub"))
            .to_request()).await;
        // Spec says: return success if subscription not found
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
    }

    // ── User credit balance routes ────────────────────────────────────────────

    #[actix_web::test]
    async fn test_get_user_credits_returns_money_only() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(get_user_credits),
        )
        .await;

        let user_id = format!("credits_user_{}", mongodb::bson::oid::ObjectId::new().to_hex());
        test::call_service(&app, test::TestRequest::post().uri("/users")
            .set_json(serde_json::json!({"id": &user_id, "userType": "individual"}))
            .to_request()).await;

        let resp = test::call_service(&app, test::TestRequest::get()
            .uri(&format!("/users/{user_id}/credits")).to_request()).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        let credits = body["credits"].as_array().expect("credits array");
        assert_eq!(credits.len(), 1);
        assert_eq!(credits[0]["creditType"], "money");
        assert_eq!(credits[0]["balance"], 0.0);
        assert_eq!(credits[0]["hourly"], 0.0);
    }

    #[actix_web::test]
    async fn test_get_user_credits_returns_404_for_missing_user() {
        let client = test_client().await;
        let app = test::init_service(
            App::new().app_data(client).service(get_user_credits),
        )
        .await;

        let resp = test::call_service(&app, test::TestRequest::get()
            .uri("/users/no_such_user/credits").to_request()).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_get_user_credit_by_type_returns_money() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(get_user_credit_by_type),
        )
        .await;

        let user_id = format!("credit_type_user_{}", mongodb::bson::oid::ObjectId::new().to_hex());
        test::call_service(&app, test::TestRequest::post().uri("/users")
            .set_json(serde_json::json!({"id": &user_id, "userType": "individual"}))
            .to_request()).await;

        let resp = test::call_service(&app, test::TestRequest::get()
            .uri(&format!("/users/{user_id}/credits/money")).to_request()).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["creditType"], "money");
        assert_eq!(body["balance"], 0.0);
    }

    #[actix_web::test]
    async fn test_get_user_credit_by_type_returns_404_for_missing_user() {
        let client = test_client().await;
        let app = test::init_service(
            App::new().app_data(client).service(get_user_credit_by_type),
        )
        .await;

        let resp = test::call_service(&app, test::TestRequest::get()
            .uri("/users/ghost_user/credits/money").to_request()).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_get_user_credit_by_type_returns_404_for_missing_resource() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(get_user_credit_by_type),
        )
        .await;

        let user_id = format!("no_resource_user_{}", mongodb::bson::oid::ObjectId::new().to_hex());
        test::call_service(&app, test::TestRequest::post().uri("/users")
            .set_json(serde_json::json!({"id": &user_id, "userType": "individual"}))
            .to_request()).await;

        let resp = test::call_service(&app, test::TestRequest::get()
            .uri(&format!("/users/{user_id}/credits/oil")).to_request()).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }
}
