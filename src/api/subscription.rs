use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::subscription::{Subscription, SubscriptionType};

fn default_money() -> String {
    "money".to_string()
}

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetSubscriptionResponse {
    id: String,
    sender: String,
    receiver: String,
    value: f32,
    subscription_type: SubscriptionType,
    priority: u32,
    credit_type: String,
}

impl GetSubscriptionResponse {
    pub(crate) fn new(sender: String, subscription: Subscription) -> Self {
        Self {
            id: subscription.id().to_string(),
            sender,
            receiver: subscription.receiver().to_string(),
            value: subscription.value(),
            subscription_type: subscription.subscription_type(),
            priority: subscription.priority(),
            credit_type: subscription.credit_type().to_string(),
        }
    }
}
