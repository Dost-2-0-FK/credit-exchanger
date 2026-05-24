use serde::{Deserialize, Serialize};

use crate::domain::subscription::{Subscription, SubscriptionType};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateSubscriptionRequest {
    receiver: u32,
    value: f32,
    subscription_type: SubscriptionType,
    priority: u32,
}

impl CreateSubscriptionRequest {
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetSubscriptionResponse {
    id: String,
    receiver: u32,
    value: f32,
    subscription_type: SubscriptionType,
    priority: u32,
}

impl From<Subscription> for GetSubscriptionResponse {
    fn from(subscription: Subscription) -> Self {
        Self {
            id: subscription.id().to_string(),
            receiver: subscription.receiver(),
            value: subscription.value(),
            subscription_type: subscription.subscription_type(),
            priority: subscription.priority(),
        }
    }
}
