use std::{ops::Sub, sync::Arc};

use derive_more::Constructor;
use serde::Serialize;

use crate::db::subscription::Subscription as DbSubscription;
use crate::domain::subscription::Subscription;

#[derive(Serialize, Constructor)]
pub(crate) struct Credit {
    total: f32,
    last_day_average: f32,
    subscriptions: Vec<Arc<Subscription>>,
    history: Vec<f32>,
}

impl Credit {
    pub(crate) fn hourly(&self, incomings: Vec<f32>) -> () {
        let subscription_sum: f32 = self
            .subscriptions
            .iter()
            .map(|sub| sub.calc(self.last_day_average))
            .sum();
        let incoming_sum: f32 = incomings.iter().sum();

        let _ = incoming_sum - subscription_sum;
    }
}

// Added a mapping function to convert between domain::Credit and db::Credit
// impl From<Credit> for crate::db::credit::Credit {
//     fn from(domain_credit: Credit) -> Self {
//         crate::db::credit::Credit::new(
//             domain_credit.total,
//             domain_credit.last_day_average,
//             domain_credit
//                 .subscriptions
//                 .iter()
//                 .map(|domain_subscription| domain_subscription.id().to_string())
//                 .collect(),
//             domain_credit.history,
//         )
//     }
// }
