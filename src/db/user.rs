use std::{collections::HashMap, sync::Arc};

use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::{
    api::user::{ListUser, User as ApiUser}, db::{
        base_user::BaseUser, bloc_user::BlocUser, error::Result, individual_user::IndividualUser,
        mongo_client::MongoClient, unit_user::UnitUser, zone_user::ZoneUser,
    }, domain::{self, subscription::Subscription},
};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum User {
    Bloc(BaseUser<BlocUser>),
    Zone(BaseUser<ZoneUser>),
    Individual(BaseUser<IndividualUser>),
    Unit(BaseUser<UnitUser>),
}

impl User {
    pub(crate) fn from_api_user(db_id: String, user: ApiUser) -> Self {
        match user {
            ApiUser::Bloc(user) => Self::Bloc(BaseUser::new(
                db_id,
                user.id().to_string(),
                BlocUser::from_domain(user.role()),
                user.credit().as_ref().clone(),
            )),
            ApiUser::Zone(user) => Self::Zone(BaseUser::new(
                db_id,
                user.id().to_string(),
                ZoneUser::from_domain(user.role()),
                user.credit().as_ref().clone(),
            )),
            ApiUser::Individual(user) => Self::Individual(BaseUser::new(
                db_id,
                user.id().to_string(),
                IndividualUser::from_domain(user.role()),
                user.credit().as_ref().clone(),
            )),
            ApiUser::Unit(user) => Self::Unit(BaseUser::new(
                db_id,
                user.id().to_string(),
                UnitUser::from_domain(user.role()),
                user.credit().as_ref().clone(),
            )),
        }
    }

    pub(crate) fn into_api_user(self) -> ApiUser {
        match self {
            Self::Bloc(user) => ApiUser::Bloc(domain::base_user::BaseUser::<
                domain::bloc_user::BlocUser,
            >::new(
                user.role.resources,
                &user.id,
                Arc::new(user.credit),
            )),
            Self::Zone(user) => ApiUser::Zone(domain::base_user::BaseUser::<
                domain::zone_user::ZoneUser,
            >::new(
                user.role.resources,
                &user.id,
                Arc::new(user.credit),
            )),
            Self::Individual(user) => {
                ApiUser::Individual(domain::base_user::BaseUser::<
                    domain::individual_user::IndividualUser,
                >::new(&user.id, Arc::new(user.credit)))
            }
            Self::Unit(user) => ApiUser::Unit(domain::base_user::BaseUser::<
                domain::unit_user::UnitUser,
            >::new(
                user.role.resources,
                &user.id,
                Arc::new(user.credit),
            )),
        }
    }

    pub(crate) fn is_unit(&self) -> bool {
        matches!(self, Self::Unit(_))
    }

    pub(crate) fn is_individual(&self) -> bool {
        matches!(self, Self::Individual(_))
    }

    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Bloc(user) => &user.id,
            Self::Zone(user) => &user.id,
            Self::Individual(user) => &user.id,
            Self::Unit(user) => &user.id,
        }
    }

    pub(crate) fn credit_mut(&mut self) -> &mut domain::credit::Credit {
        match self {
            Self::Bloc(user) => &mut user.credit,
            Self::Zone(user) => &mut user.credit,
            Self::Individual(user) => &mut user.credit,
            Self::Unit(user) => &mut user.credit,
        }
    }

    pub(crate) fn credit(&self) -> &domain::credit::Credit {
        match self {
            Self::Bloc(user) => &user.credit,
            Self::Zone(user) => &user.credit,
            Self::Individual(user) => &user.credit,
            Self::Unit(user) => &user.credit,
        }
    }

    pub(crate) fn credit_total(&self) -> f32 {
        match self {
            Self::Bloc(user) => user.credit.total(),
            Self::Zone(user) => user.credit.total(),
            Self::Individual(user) => user.credit.total(),
            Self::Unit(user) => user.credit.total(),
        }
    }

    pub(crate) fn resources_mut(
        &mut self,
    ) -> Option<&mut HashMap<String, domain::credit::Credit>> {
        match self {
            Self::Bloc(user) => Some(&mut user.role.resources),
            Self::Zone(user) => Some(&mut user.role.resources),
            Self::Unit(user) => Some(&mut user.role.resources),
            Self::Individual(_) => None,
        }
    }

    pub(crate) fn resources(&self) -> Option<&HashMap<String, domain::credit::Credit>> {
        match self {
            Self::Bloc(user) => Some(&user.role.resources),
            Self::Zone(user) => Some(&user.role.resources),
            Self::Unit(user) => Some(&user.role.resources),
            Self::Individual(_) => None,
        }
    }

    pub(crate) fn credit_subscriptions(&self) -> &[std::sync::Arc<Subscription>] {
        match self {
            Self::Bloc(user) => user.credit.subscriptions(),
            Self::Zone(user) => user.credit.subscriptions(),
            Self::Individual(user) => user.credit.subscriptions(),
            Self::Unit(user) => user.credit.subscriptions(),
        }
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        let mut subscriptions = self
            .credit_subscriptions()
            .iter()
            .map(|subscription| subscription.as_ref().clone())
            .collect::<Vec<_>>();

        if let Some(resources) = self.resources() {
            for credit in resources.values() {
                subscriptions.extend(
                    credit
                        .subscriptions()
                        .iter()
                        .map(|subscription| subscription.as_ref().clone()),
                );
            }
        }

        subscriptions
    }
}

pub(crate) struct MoneyBookingOutcome {
    pub(crate) sender_reached_zero: bool,
}

impl UsersRepository {
    pub(crate) async fn book_resource(
        &self,
        sender_id: &str,
        receiver_id: &str,
        resource_name: &str,
        value: f32,
    ) -> Result<()> {
        if value <= 0.0 {
            return Err(crate::db::error::Error::Validation(
                "Booking value must be greater than zero",
            ));
        }

        let mut sender = self
            .get_db_user(sender_id)
            .await?
            .ok_or(crate::db::error::Error::NotFound("sender user"))?;
        let mut receiver = self
            .get_db_user(receiver_id)
            .await?
            .ok_or(crate::db::error::Error::NotFound("receiver user"))?;

        let sender_resources = sender
            .resources_mut()
            .ok_or(crate::db::error::Error::Validation(
                "Sender does not have resource credits",
            ))?;
        let sender_resource =
            sender_resources
                .get_mut(resource_name)
                .ok_or(crate::db::error::Error::NotFound("resource credit"))?;

        if sender_resource.total() < value {
            return Err(crate::db::error::Error::Validation(
                "Insufficient resource credit for booking",
            ));
        }
        sender_resource.apply_amount(-value);
        sender_resource.add_transfer_history_entry(domain::credit::TransferHistoryEntry::new(
            -value,
            sender_id.to_string(),
            receiver_id.to_string(),
            "booking".to_string(),
        ));

        let receiver_resources =
            receiver
                .resources_mut()
                .ok_or(crate::db::error::Error::Validation(
                    "Receiver does not have resource credits",
                ))?;
        let receiver_resource = receiver_resources
            .entry(resource_name.to_string())
            .or_insert_with(|| domain::credit::Credit::new(0.0, 0.0, vec![], vec![]));
        receiver_resource.apply_amount(value);
        receiver_resource.add_transfer_history_entry(domain::credit::TransferHistoryEntry::new(
            value,
            sender_id.to_string(),
            receiver_id.to_string(),
            "booking".to_string(),
        ));

        self.replace_db_user(&sender).await?;
        self.replace_db_user(&receiver).await?;

        Ok(())
    }

    pub(crate) async fn add_subscription_to_user_resource_credit(
        &self,
        user_id: &str,
        resource_name: &str,
        subscription: Subscription,
    ) -> Result<Option<ApiUser>> {
        let Some(mut user) = self.get_db_user(user_id).await? else {
            return Ok(None);
        };

        let resources = match user.resources_mut() {
            Some(r) => r,
            None => {
                return Err(crate::db::error::Error::Validation(
                    "User does not have resource credits",
                ))
            }
        };

        resources
            .entry(resource_name.to_string())
            .or_insert_with(|| domain::credit::Credit::new(0.0, 0.0, vec![], vec![]))
            .add_subscription(Arc::new(subscription));

        self.replace_db_user(&user).await?;
        Ok(Some(user.into_api_user()))
    }
}

#[derive(Constructor)]
pub(crate) struct UsersRepository {
    db: MongoClient,
}

impl UsersRepository {
    pub(crate) async fn get_user(&self, user_id: &str) -> Result<Option<ApiUser>> {
        let user_doc = self
            .db
            .get_document_by_field("users", "_id", user_id)
            .await?;
        let Some(user_doc) = user_doc else {
            return Ok(None);
        };

        let user = mongodb::bson::from_document::<User>(user_doc)?;
        Ok(Some(user.into_api_user()))
    }

    pub(crate) async fn get_db_user(&self, user_id: &str) -> Result<Option<User>> {
        let user_doc = self
            .db
            .get_document_by_field("users", "_id", user_id)
            .await?;
        let Some(user_doc) = user_doc else {
            return Ok(None);
        };

        let user = mongodb::bson::from_document::<User>(user_doc)?;
        Ok(Some(user))
    }

    pub(crate) async fn list_users(&self) -> Result<Vec<ListUser>> {
        let user_docs = self.db.list_documents("users").await?;
        user_docs
            .into_iter()
            .map(|user_doc| {
                mongodb::bson::from_document::<User>(user_doc)
                    .map(User::into_api_user).map(Into::into)
                    .map_err(Into::into)
            })
            .collect()
    }

    pub(crate) async fn list_db_users(&self) -> Result<Vec<User>> {
        let user_docs = self.db.list_documents("users").await?;
        user_docs
            .into_iter()
            .map(|user_doc| mongodb::bson::from_document::<User>(user_doc).map_err(Into::into))
            .collect()
    }

    pub(crate) async fn insert_user(&self, user: ApiUser) -> Result<ApiUser> {
        let db_user = User::from_api_user(user.id().to_string(), user);
        let doc = mongodb::bson::to_document(&db_user)?;
        match self.db.insert_document("users", doc).await {
            Ok(()) => {}
            Err(crate::db::error::Error::DbError(err)) if err.to_string().contains("E11000") => {
                return Err(crate::db::error::Error::Validation("User already exists"));
            }
            Err(err) => return Err(err),
        }

        Ok(db_user.into_api_user())
    }

    pub(crate) async fn replace_db_user(&self, user: &User) -> Result<()> {
        let doc = mongodb::bson::to_document(user)?;
        self.db
            .replace_document_by_field("users", "_id", user.id(), doc)
            .await
    }

    pub(crate) async fn add_subscription_to_user_credit(
        &self,
        user_id: &str,
        subscription: Subscription,
    ) -> Result<Option<ApiUser>> {
        let Some(mut user) = self.get_db_user(user_id).await? else {
            return Ok(None);
        };

        user.credit_mut().add_subscription(Arc::new(subscription));
        self.replace_db_user(&user).await?;

        Ok(Some(user.into_api_user()))
    }

    pub(crate) async fn update_unit_credit_last_day_average(
        &self,
        user_id: &str,
        credit_type: &str,
        last_day_average: f32,
    ) -> Result<Option<ApiUser>> {
        let Some(mut user) = self.get_db_user(user_id).await? else {
            return Ok(None);
        };

        if credit_type == "money" {
            user.credit_mut().set_last_day_average(last_day_average);
        } else {
            let Some(resource_credit) = user
                .resources_mut()
                .and_then(|resources| resources.get_mut(credit_type))
            else {
                return Err(crate::db::error::Error::NotFound("credit type"));
            };

            resource_credit.set_last_day_average(last_day_average);
        }

        self.replace_db_user(&user).await?;
        Ok(Some(user.into_api_user()))
    }

    pub(crate) async fn book_money(
        &self,
        sender_id: &str,
        receiver_id: &str,
        value: f32,
    ) -> Result<MoneyBookingOutcome> {
        if value <= 0.0 {
            return Err(crate::db::error::Error::Validation(
                "Booking value must be greater than zero",
            ));
        }

        let mut sender = self
            .get_db_user(sender_id)
            .await?
            .ok_or(crate::db::error::Error::NotFound("sender user"))?;
        let mut receiver = self
            .get_db_user(receiver_id)
            .await?
            .ok_or(crate::db::error::Error::NotFound("receiver user"))?;

        let sender_total = sender.credit_total();
        if sender_total < value {
            return Err(crate::db::error::Error::Validation(
                "Insufficient credit for booking",
            ));
        }

        let updated_sender_total = sender_total - value;

        sender.credit_mut().apply_amount(-value);
        sender
            .credit_mut()
            .add_transfer_history_entry(domain::credit::TransferHistoryEntry::new(
                -value,
                sender_id.to_string(),
                receiver_id.to_string(),
                "booking".to_string(),
            ));

        receiver.credit_mut().apply_amount(value);
        receiver
            .credit_mut()
            .add_transfer_history_entry(domain::credit::TransferHistoryEntry::new(
                value,
                sender_id.to_string(),
                receiver_id.to_string(),
                "booking".to_string(),
            ));

        self.replace_db_user(&sender).await?;
        self.replace_db_user(&receiver).await?;

        Ok(MoneyBookingOutcome {
            sender_reached_zero: sender.is_individual()
                && sender_total > 0.0
                && updated_sender_total.abs() < f32::EPSILON,
        })
    }

    /// Returns all incoming and outgoing subscriptions across all credits for a user.
    /// Returns `None` if the user does not exist.
    pub(crate) async fn list_user_subscriptions(
        &self,
        user_id: &str,
        sender_filter: Option<&str>,
        receiver_filter: Option<&str>,
    ) -> Result<Option<Vec<(String, Subscription)>>> {
        let users = self.list_db_users().await?;
        if !users.iter().any(|user| user.id() == user_id) {
            return Ok(None);
        }

        let subscriptions = users
            .into_iter()
            .flat_map(|user| {
                let sender = user.id().to_string();
                user.subscriptions()
                    .into_iter()
                    .map(move |subscription| (sender.clone(), subscription))
            })
            .filter(|(sender, subscription)| {
                (sender == user_id || subscription.receiver() == user_id)
                    && sender_filter.is_none_or(|filter| sender == filter)
                    && receiver_filter
                        .is_none_or(|filter| subscription.receiver() == filter)
            })
            .collect();

        Ok(Some(subscriptions))
    }

    pub(crate) async fn find_subscription_sender(
        &self,
        subscription_id: &str,
    ) -> Result<Option<String>> {
        Ok(self.list_db_users().await?.into_iter().find_map(|user| {
            user.subscriptions()
                .iter()
                .any(|subscription| subscription.id() == subscription_id)
                .then(|| user.id().to_string())
        }))
    }

    /// Remove a subscription (from money credit or any resource credit) for a user.
    /// Returns `None` if the user does not exist.
    /// Returns `Ok(Some(false))` if the subscription was not found (spec: still success).
    pub(crate) async fn remove_user_subscription(
        &self,
        user_id: &str,
        subscription_id: &str,
    ) -> Result<Option<bool>> {
        let Some(mut user) = self.get_db_user(user_id).await? else {
            return Ok(None);
        };

        let mut removed = user.credit_mut().remove_subscription(subscription_id);

        if !removed {
            if let Some(resources) = user.resources_mut() {
                for credit in resources.values_mut() {
                    if credit.remove_subscription(subscription_id) {
                        removed = true;
                        break;
                    }
                }
            }
        }

        if removed {
            self.replace_db_user(&user).await?;
        }

        Ok(Some(removed))
    }
}
