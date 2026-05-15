use std::ops::Sub;

use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(Debug, Serialize, Deserialize, EnumString, Clone, Copy)]
#[strum(serialize_all = "camelCase")]
pub(crate) enum SubscriptionType {
    Sr,
    Contract,
}

#[derive(Serialize, Constructor, Clone)]
pub(crate) struct Subscription {
    id: String,
    receiver: u32, // the receivers unique id
    value: f32,    // value in percentage, might be positive or negative
    subscription_type: SubscriptionType,
    // TODO check if priority is always positive int
    priority: u32,
}

impl Subscription {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn calc(&self, last_day_average: f32) -> f32 {
        last_day_average * self.value / 100.0
    }
}

impl From<Subscription> for crate::db::subscription::Subscription {
    fn from(domain_subscription: Subscription) -> Self {
        crate::db::subscription::Subscription::new(
            domain_subscription.id,
            domain_subscription.receiver,
            domain_subscription.value,
            domain_subscription.subscription_type,
            domain_subscription.priority,
        )
    }
}
