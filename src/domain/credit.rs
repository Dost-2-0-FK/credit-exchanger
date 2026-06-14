use std::sync::Arc;

use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::subscription::{Subscription, SubscriptionType};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EvaluatedSubscription {
    id: String,
    receiver: String,
    amount: f32,
    subscription_type: SubscriptionType,
    priority: u32,
}

#[allow(dead_code)]
impl EvaluatedSubscription {
    fn from_subscription(subscription: &Subscription, amount: f32) -> Self {
        Self {
            id: subscription.id().to_string(),
            receiver: subscription.receiver().to_string(),
            amount,
            subscription_type: subscription.subscription_type(),
            priority: subscription.priority(),
        }
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn receiver(&self) -> &str {
        &self.receiver
    }

    pub(crate) fn amount(&self) -> f32 {
        self.amount
    }

    pub(crate) fn subscription_type(&self) -> SubscriptionType {
        self.subscription_type
    }

    pub(crate) fn priority(&self) -> u32 {
        self.priority
    }
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct CreditEvaluation {
    booked_subscriptions: Vec<EvaluatedSubscription>,
    sr_overflows: Vec<f32>,
    hit_zero: bool,
}

#[allow(dead_code)]
impl CreditEvaluation {
    pub(crate) fn booked_subscriptions(&self) -> &[EvaluatedSubscription] {
        &self.booked_subscriptions
    }

    pub(crate) fn sr_overflows(&self) -> &[f32] {
        &self.sr_overflows
    }

    pub(crate) fn hit_zero(&self) -> bool {
        self.hit_zero
    }
}

#[derive(Serialize, Deserialize, Constructor, Clone, ToSchema)]
pub(crate) struct Credit {
    total: f32,
    last_day_average: f32,
    #[schema(value_type = Vec<Subscription>)]
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

    pub(crate) fn add_subscription(&mut self, subscription: Arc<Subscription>) {
        self.subscriptions.push(subscription);
    }

    /// Returns true if a subscription with the given id was found and removed.
    pub(crate) fn remove_subscription(&mut self, subscription_id: &str) -> bool {
        let before = self.subscriptions.len();
        self.subscriptions.retain(|s| s.id() != subscription_id);
        self.subscriptions.len() < before
    }

    pub(crate) fn apply_amount(&mut self, amount: f32) {
        self.total += amount;
    }

    #[allow(dead_code)]
    pub(crate) fn hourly(&mut self, incomings: Vec<f32>) -> f32 {
        let subscription_sum: f32 = self
            .subscriptions
            .iter()
            .map(|sub| sub.calc(self.last_day_average))
            .sum();
        let incoming_sum: f32 = incomings.iter().sum();
        let hourly_income = incoming_sum - subscription_sum;

        self.history.push(hourly_income);

        hourly_income
    }

    #[allow(dead_code)]
    pub(crate) fn calc_avrg(&mut self) -> f32 {
        let average = if self.history.is_empty() {
            0.0
        } else {
            self.history.iter().sum::<f32>() / self.history.len() as f32
        };

        self.last_day_average = average;
        self.history.clear();

        average
    }

    #[allow(dead_code)]
    pub(crate) fn evaluate(&mut self) -> CreditEvaluation {
        let mut subscriptions = self.subscriptions.clone();
        subscriptions.sort_by(|left, right| {
            subscription_type_rank(left.subscription_type())
                .cmp(&subscription_type_rank(right.subscription_type()))
                .then_with(|| right.priority().cmp(&left.priority()))
                .then_with(|| left.id().cmp(right.id()))
        });

        let started_positive = self.total > 0.0;
        let mut evaluation = CreditEvaluation::default();

        for subscription in subscriptions {
            let amount = subscription.calc(self.last_day_average);

            if amount > 0.0 && self.total + f32::EPSILON < amount {
                if matches!(subscription.subscription_type(), SubscriptionType::Sr) {
                    evaluation.sr_overflows.push(amount - self.total);
                }
                continue;
            }

            self.total -= amount;
            evaluation
                .booked_subscriptions
                .push(EvaluatedSubscription::from_subscription(&subscription, amount));
        }

        evaluation.hit_zero = started_positive && self.total.abs() < f32::EPSILON;

        evaluation
    }
}

#[allow(dead_code)]
fn subscription_type_rank(subscription_type: SubscriptionType) -> u8 {
    match subscription_type {
        SubscriptionType::Sr => 0,
        SubscriptionType::Contract => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::subscription::Subscription;

    fn subscription(
        id: &str,
        receiver: &str,
        value: f32,
        subscription_type: SubscriptionType,
        priority: u32,
    ) -> Arc<Subscription> {
        Arc::new(Subscription::new(
            id.to_string(),
            receiver.to_string(),
            value,
            subscription_type,
            priority,
            "money".to_string(),
        ))
    }

    #[test]
    fn test_hourly_records_net_income_in_history() {
        let mut credit = Credit::new(
            25.0,
            100.0,
            vec![subscription("sub_1", "receiver", 10.0, SubscriptionType::Contract, 1)],
            vec![2.0],
        );

        let hourly_income = credit.hourly(vec![30.0, 25.0]);

        assert_eq!(hourly_income, 45.0);
        assert_eq!(credit.history(), &[2.0, 45.0]);
    }

    #[test]
    fn test_calc_avrg_updates_last_day_average_and_clears_history() {
        let mut credit = Credit::new(25.0, 0.0, vec![], vec![10.0, 20.0, 30.0]);

        let average = credit.calc_avrg();

        assert_eq!(average, 20.0);
        assert_eq!(credit.last_day_average(), 20.0);
        assert!(credit.history().is_empty());
    }

    #[test]
    fn test_evaluate_books_sr_before_contract() {
        let mut credit = Credit::new(
            15.0,
            100.0,
            vec![
                subscription("contract", "contract_receiver", 10.0, SubscriptionType::Contract, 10),
                subscription("sr", "sr_receiver", 10.0, SubscriptionType::Sr, 1),
            ],
            vec![],
        );

        let evaluation = credit.evaluate();

        let booked_ids: Vec<&str> = evaluation
            .booked_subscriptions()
            .iter()
            .map(EvaluatedSubscription::id)
            .collect();
        assert_eq!(booked_ids, vec!["sr"]);
        assert!(evaluation.sr_overflows().is_empty());
        assert!(!evaluation.hit_zero());
        assert_eq!(credit.total(), 5.0);
    }

    #[test]
    fn test_evaluate_records_sr_overflow_when_booking_cannot_be_covered() {
        let mut credit = Credit::new(
            5.0,
            100.0,
            vec![subscription("sr", "receiver", 10.0, SubscriptionType::Sr, 1)],
            vec![],
        );

        let evaluation = credit.evaluate();

        assert!(evaluation.booked_subscriptions().is_empty());
        assert_eq!(evaluation.sr_overflows(), &[5.0]);
        assert_eq!(credit.total(), 5.0);
    }

    #[test]
    fn test_evaluate_marks_when_credit_hits_zero() {
        let mut credit = Credit::new(
            10.0,
            100.0,
            vec![subscription("contract", "receiver", 10.0, SubscriptionType::Contract, 1)],
            vec![],
        );

        let evaluation = credit.evaluate();

        assert!(evaluation.hit_zero());
        assert_eq!(credit.total(), 0.0);
    }
}
