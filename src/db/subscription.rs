use derive_more::Constructor;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::{
    db::mongo_client::MongoClient,
    domain::{self, subscription::SubscriptionType},
};

use crate::db::error::Result;

fn default_credit_type() -> String {
    "money".to_string()
}

#[derive(Debug, Serialize, Deserialize, Constructor)]
pub(crate) struct Subscription {
    #[serde(rename = "_id")]
    id: ObjectId,
    receiver: String,
    value: f32,    // value in percentage, might be positive or negative
    subscription_type: SubscriptionType,
    // TODO check if priority is always positive int
    priority: u32,
    #[serde(default = "default_credit_type")]
    credit_type: String,
}

#[derive(Constructor)]
pub(crate) struct SubscriptionsRepository {
    db: MongoClient,
}

impl SubscriptionsRepository {
    pub(crate) async fn get_subscription(
        &self,
        id: &ObjectId,
    ) -> Result<Option<domain::subscription::Subscription>> {
        // Query the Subscription document from the database
        let subscription_doc = self
            .db
            .get_document_by_object_id("subscription", id)
            .await?;

        // Deserialize the Subscription document
        let Some(subscription_doc) = subscription_doc else {
            return Ok(None);
        };
        let db_subscription = mongodb::bson::from_document::<Subscription>(subscription_doc)?;

        // Build and return the domain Subscription object
        let subscription = domain::subscription::Subscription::new(
            db_subscription.id.to_hex(),
            db_subscription.receiver,
            db_subscription.value,
            db_subscription.subscription_type,
            db_subscription.priority,
            db_subscription.credit_type,
        );
        Ok(Some(subscription))
    }

    pub(crate) async fn insert_subscription(
        &self,
        subscription: domain::subscription::Subscription,
    ) -> Result<Option<domain::subscription::Subscription>> {
        // Persist the domain ID as MongoDB `_id`.
        let db_subscription = Subscription::new(
            ObjectId::parse_str(subscription.id())?,
            subscription.receiver().to_string(),
            subscription.value(),
            subscription.subscription_type(),
            subscription.priority(),
            subscription.credit_type().to_string(),
        );

        // Convert to MongoDB document and insert
        let doc = mongodb::bson::to_document(&db_subscription)?;
        self.db.insert_document("subscription", doc).await?;
        Ok(Some(subscription))
    }
}
