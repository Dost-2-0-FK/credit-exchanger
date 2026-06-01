use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(Debug, Serialize, Deserialize, EnumString, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub(crate) enum SubscriptionType {
    Sr,
    Contract,
}

fn default_credit_type() -> String {
    "money".to_string()
}

#[derive(Serialize, Deserialize, Constructor, Clone)]
pub(crate) struct Subscription {
    id: String,
    receiver: String,
    value: f32,    // value in percentage, might be positive or negative
    subscription_type: SubscriptionType,
    // TODO check if priority is always positive int
    priority: u32,
    #[serde(default = "default_credit_type")]
    credit_type: String,
}

impl Subscription {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn receiver(&self) -> &str {
        &self.receiver
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

    pub(crate) fn credit_type(&self) -> &str {
        &self.credit_type
    }

    #[allow(dead_code)]
    pub(crate) fn calc(&self, last_day_average: f32) -> f32 {
        last_day_average * self.value / 100.0
    }
}
