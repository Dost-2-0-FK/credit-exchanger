use serde::Serialize;
use strum::EnumString;

#[derive(Debug, Serialize, EnumString)]
#[strum(serialize_all = "camelCase")]
enum SubscriptionType {
    Sr,
    Contract,
}

#[derive(Serialize)]
pub(crate) struct Subscription {
    id: String,
    receiver: u32, // the receivers unique id
    value: f32,    // value in percentage, might be positive or negative
    subscription_type: SubscriptionType,
    // TODO check if priority is always positive int
    priority: u32,
}

impl Subscription {
    pub(crate) fn calc(&self, last_day_average: f32) -> f32 {
        last_day_average * self.value / 100.0
    }
}
