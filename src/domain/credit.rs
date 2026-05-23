use std::sync::Arc;

use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::domain::subscription::Subscription;

#[derive(Serialize, Deserialize, Constructor, Clone)]
pub(crate) struct Credit {
    total: f32,
    last_day_average: f32,
    subscriptions: Vec<Arc<Subscription>>,
    history: Vec<f32>,
}

impl Credit {
    pub(crate) fn total(&self) -> f32 {
        self.total
    }

    pub(crate) fn last_day_average(&self) -> f32 {
        self.last_day_average
    }

    pub(crate) fn subscriptions(&self) -> &[Arc<Subscription>] {
        &self.subscriptions
    }

    pub(crate) fn history(&self) -> &[f32] {
        &self.history
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
