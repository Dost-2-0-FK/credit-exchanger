use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::{
    db::mongo_client::MongoClient,
    domain::{self, subscription::SubscriptionType},
};

use crate::db::error::Result;

#[derive(Debug, Serialize, Deserialize, Constructor)]
pub(crate) struct Subscription {
    id: String,
    receiver: u32, // the receivers unique id
    value: f32,    // value in percentage, might be positive or negative
    subscription_type: SubscriptionType,
    // TODO check if priority is always positive int
    priority: u32,
}

pub(crate) struct SubscriptionsRepository {
    db: MongoClient,
}

impl SubscriptionsRepository {
    pub(crate) async fn get_subscription(
        &self,
        id: &str,
    ) -> Result<Option<domain::subscription::Subscription>> {
        // Query the Subscription document from the database
        let subscription_doc = self.db.get_document("subscription", id).await?;

        // Deserialize the Subscription document
        let Some(subscription_doc) = subscription_doc else {
            return Ok(None);
        };
        let db_subscription = mongodb::bson::from_document::<Subscription>(subscription_doc)?;

        // Build and return the domain Subscription object
        let subscription = domain::subscription::Subscription::new(
            db_subscription.id,
            db_subscription.receiver,
            db_subscription.value,
            db_subscription.subscription_type,
            db_subscription.priority,
        );
        Ok(Some(subscription))
    }
}
