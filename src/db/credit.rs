use std::sync::Arc;

use derive_more::Constructor;
use futures_util::{StreamExt, TryStreamExt as _, stream};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::db::subscription::SubscriptionsRepository;

use crate::{db::mongo_client::MongoClient, domain};

use crate::db::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize, Constructor)]
pub(crate) struct Credit {
    #[serde(rename = "_id")]
    id: ObjectId,
    total: f32,
    last_day_average: f32,
    subscription_ids: Vec<String>,
    history: Vec<f32>,
    #[serde(default)]
    transfer_history: Vec<domain::credit::TransferHistoryEntry>,
}

#[derive(Constructor)]
pub(crate) struct CreditRepository {
    db: MongoClient,
    subscription_repository: SubscriptionsRepository,
}

impl CreditRepository {
    pub(crate) async fn get_credit(&self, id: &str) -> Result<Option<domain::credit::Credit>> {
        let object_id = ObjectId::parse_str(id)?;

        // Query the Credit document from the database
        let credit_doc = self
            .db
            .get_document_by_object_id("credit", &object_id)
            .await?;

        // Deserialize the Credit document
        let Some(credit_doc) = credit_doc else {
            return Ok(None);
        };
        let db_credit = mongodb::bson::from_document::<Credit>(credit_doc)?;

        // Query the Subscription repository for credit subscriptions
        let domain_subscriptions = stream::iter(db_credit.subscription_ids.iter())
            .then(async |id| -> Result<_> {
                let object_id = mongodb::bson::oid::ObjectId::parse_str(id)?;
                let subscription = self
                    .subscription_repository
                    .get_subscription(&object_id)
                    .await?
                    .map(Arc::new)
                    .ok_or(Error::NotFound("matching subscription for credit"))?;
                Ok(subscription)
            })
            .try_collect::<Vec<_>>()
            .await?;

        // Build and return the domain Credit object
        let credit = domain::credit::Credit::with_transfer_history(
            db_credit.total,
            db_credit.last_day_average,
            domain_subscriptions,
            db_credit.history,
            db_credit.transfer_history,
        );
        Ok(Some(credit))
    }

    pub(crate) async fn insert_credit(&self, credit: domain::credit::Credit) -> Result<String> {
        let id = ObjectId::new();
        let db_credit = Credit::new(
            id,
            credit.total(),
            credit.last_day_average(),
            credit
                .subscriptions()
                .iter()
                .map(|domain_subscription| domain_subscription.id().to_string())
                .collect(),
            credit.history().to_vec(),
            credit.transfer_history().to_vec(),
        );

        let doc = mongodb::bson::to_document(&db_credit)?;
        self.db.insert_document("credit", doc).await?;
        Ok(db_credit.id.to_hex())
    }
}
