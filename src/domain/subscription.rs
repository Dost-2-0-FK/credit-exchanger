use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(Debug, Serialize, Deserialize, EnumString, Clone, Copy)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub(crate) enum SubscriptionType {
    Sr,
    Contract,
}

#[derive(Serialize, Deserialize, Constructor, Clone)]
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

    pub(crate) fn receiver(&self) -> u32 {
        self.receiver
    }

    pub(crate) fn value(&self) -> f32 {
        self.value
    }

    pub(crate) fn subscription_type(&self) -> SubscriptionType {
        self.subscription_type
    }

    pub(crate) fn priority(&self) -> u32 {
        self.priority
    }

    pub(crate) fn calc(&self, last_day_average: f32) -> f32 {
        last_day_average * self.value / 100.0
    }
}
