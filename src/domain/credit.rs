use std::sync::Arc;

use serde::Serialize;

use crate::domain::subscription::Subscription;

#[derive(Serialize)]
pub(crate) struct Credit {
    total: f32,
    last_day_average: f32,
    subscriptions: Vec<Arc<Subscription>>,
    history: Vec<f32>,
}

impl Credit {
    pub(crate) fn new(
        total: f32,
        last_day_average: f32,
        subscriptions: Vec<Arc<Subscription>>,
        history: Vec<f32>,
    ) -> Self {
        Self {
            total,
            last_day_average,
            subscriptions,
            history,
        }
    }

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

