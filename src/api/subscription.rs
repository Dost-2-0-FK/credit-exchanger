use serde::{Deserialize, Serialize};

use crate::domain::subscription::{Subscription, SubscriptionType};

fn default_money() -> String {
    "money".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateSubscriptionRequest {
    receiver: String,
    value: f32,
    subscription_type: SubscriptionType,
    priority: u32,
    #[serde(default = "default_money")]
    credit_type: String,
}

impl CreateSubscriptionRequest {
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetSubscriptionResponse {
    id: String,
    receiver: String,
    value: f32,
    subscription_type: SubscriptionType,
    priority: u32,
    credit_type: String,
}

impl From<Subscription> for GetSubscriptionResponse {
    fn from(subscription: Subscription) -> Self {
        Self {
            id: subscription.id().to_string(),
            receiver: subscription.receiver().to_string(),
            value: subscription.value(),
            subscription_type: subscription.subscription_type(),
            priority: subscription.priority(),
            credit_type: subscription.credit_type().to_string(),
        }
    }
}
