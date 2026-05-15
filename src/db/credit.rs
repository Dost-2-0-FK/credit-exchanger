use std::{ops::Sub, sync::Arc};

use derive_more::Constructor;
use futures_util::{StreamExt, TryStreamExt as _, stream};
use serde::{Deserialize, Serialize};

use crate::db::subscription::{Subscription, SubscriptionsRepository};

use crate::{
    db::{self, credit, mongo_client::MongoClient},
    domain,
};

use crate::db::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize, Constructor)]
pub(crate) struct Credit {
    id: String,
    total: f32,
    last_day_average: f32,
    subscription_ids: Vec<String>,
    history: Vec<f32>,
}

struct CreditRepository {
    db: MongoClient,
    subscription_repository: SubscriptionsRepository,
}

impl CreditRepository {
    async fn get_credit(&self, id: &str) -> Result<Option<domain::credit::Credit>> {
        // Query the Credit document from the database
        let credit_doc = self.db.get_document("credit", id).await?;

        // Deserialize the Credit document
        let Some(credit_doc) = credit_doc else {
            return Ok(None);
        };
        let db_credit = mongodb::bson::from_document::<Credit>(credit_doc)?;

        // Query the Subscription repository for credit subscriptions
        let domain_subscriptions = stream::iter(db_credit.subscription_ids.iter())
            .then(async |id| -> Result<_> {
                let subscription = self
                    .subscription_repository
                    .get_subscription(id)
                    .await?
                    .map(Arc::new)
                    .ok_or(Error::NotFound("matching subscription for credit"))?;
                Ok(subscription)
            })
            .try_collect::<Vec<_>>()
            .await?;

        // Build and return the domain Credit object
        let credit = domain::credit::Credit::new(
            db_credit.total,
            db_credit.last_day_average,
            domain_subscriptions,
            db_credit.history,
        );
        Ok(Some(credit))
    }

    async fn post_credit(&self, credit: domain::credit::Credit) {
        todo!()
    }
}
