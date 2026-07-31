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
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UnbookableSrTransfer {
    receiver: String,
    amount: f32,
}

#[allow(dead_code)]
impl UnbookableSrTransfer {
    fn new(receiver: String, amount: f32) -> Self {
        Self { receiver, amount }
    }

    pub(crate) fn receiver(&self) -> &str {
        &self.receiver
    }

    pub(crate) fn amount(&self) -> f32 {
        self.amount
    }
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
    unbookable_sr_transfers: Vec<UnbookableSrTransfer>,
    hit_zero: bool,
}

#[derive(Debug, Serialize, Deserialize, Constructor, Clone, PartialEq, ToSchema)]
pub(crate) struct TransferHistoryEntry {
    value: f32,
    sender: String,
    receiver: String,
    #[serde(rename = "type")]
    transfer_type: String,
}

#[allow(dead_code)]
impl CreditEvaluation {
    pub(crate) fn booked_subscriptions(&self) -> &[EvaluatedSubscription] {
        &self.booked_subscriptions
    }

    pub(crate) fn unbookable_sr_transfers(&self) -> &[UnbookableSrTransfer] {
        &self.unbookable_sr_transfers
    }

    pub(crate) fn hit_zero(&self) -> bool {
        self.hit_zero
    }
}

#[derive(Serialize, Deserialize, Constructor, Clone, ToSchema)]
pub(crate) struct ListUserCredit {
    total: f32,
    last_day_average: f32,
}

impl From<Arc<Credit>> for ListUserCredit {
    fn from(value: Arc<Credit>) -> Self {
        Self {
            total: value.total(),
            last_day_average: value.last_day_average(),
        }
    }
}

impl From<Credit> for ListUserCredit {
    fn from(value: Credit) -> Self {
        Self {
            total: value.total(),
            last_day_average: value.last_day_average(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub(crate) struct Credit {
    total: f32,
    last_day_average: f32,
    #[schema(value_type = Vec<Subscription>)]
    #[serde(default)]
    subscriptions: Vec<Arc<Subscription>>,
    #[serde(default)]
    history: Vec<f32>,
    #[serde(default)]
    transfer_history: Vec<TransferHistoryEntry>,
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
            transfer_history: vec![],
        }
    }

    pub(crate) fn with_transfer_history(
        total: f32,
        last_day_average: f32,
        subscriptions: Vec<Arc<Subscription>>,
        history: Vec<f32>,
        transfer_history: Vec<TransferHistoryEntry>,
    ) -> Self {
        Self {
            total,
            last_day_average,
            subscriptions,
            history,
            transfer_history,
        }
    }

    pub(crate) fn total(&self) -> f32 {
        self.total
    }

    pub(crate) fn last_day_average(&self) -> f32 {
        self.last_day_average
    }

    pub(crate) fn set_last_day_average(&mut self, last_day_average: f32) {
        self.last_day_average = last_day_average;
    }

    pub(crate) fn subscriptions(&self) -> &[Arc<Subscription>] {
        &self.subscriptions
    }

    pub(crate) fn history(&self) -> &[f32] {
        &self.history
    }

    pub(crate) fn transfer_history(&self) -> &[TransferHistoryEntry] {
        &self.transfer_history
    }

    pub(crate) fn add_subscription(&mut self, subscription: Arc<Subscription>) {
        self.subscriptions.push(subscription);
    }

    pub(crate) fn add_transfer_history_entry(&mut self, entry: TransferHistoryEntry) {
        self.transfer_history.push(entry);
    }

    pub(crate) fn extend_transfer_history(&mut self, entries: Vec<TransferHistoryEntry>) {
        self.transfer_history.extend(entries);
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
        let subscriptions = sorted_subscriptions(&self.subscriptions);
        let started_positive = self.total > 0.0;
        let mut evaluation = CreditEvaluation::default();

        for subscription in subscriptions {
            let amount = subscription.calc(self.last_day_average);

            if amount > 0.0 && self.total + f32::EPSILON < amount {
                if matches!(subscription.subscription_type(), SubscriptionType::Sr) {
                    // Keep the full intended SR transfer amount for external notification.
                    evaluation.unbookable_sr_transfers.push(UnbookableSrTransfer::new(
                        subscription.receiver().to_string(),
                        amount,
                    ));
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

    pub(crate) fn evaluate_without_balance(&self) -> CreditEvaluation {
        let subscriptions = sorted_subscriptions(&self.subscriptions);
        let mut evaluation = CreditEvaluation::default();

        for subscription in subscriptions {
            let amount = subscription.calc(self.last_day_average);
            evaluation
                .booked_subscriptions
                .push(EvaluatedSubscription::from_subscription(&subscription, amount));
        }

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

fn sorted_subscriptions(subscriptions: &[Arc<Subscription>]) -> Vec<Arc<Subscription>> {
    let mut subscriptions = subscriptions.to_vec();
    subscriptions.sort_by(|left, right| {
        subscription_type_rank(left.subscription_type())
            .cmp(&subscription_type_rank(right.subscription_type()))
            .then_with(|| right.priority().cmp(&left.priority()))
            .then_with(|| left.id().cmp(right.id()))
    });
    subscriptions
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
        assert!(evaluation.unbookable_sr_transfers().is_empty());
        assert!(!evaluation.hit_zero());
        assert_eq!(credit.total(), 5.0);
    }

    #[test]
    fn test_evaluate_records_unbookable_sr_transfer_when_booking_cannot_be_covered() {
        let mut credit = Credit::new(
            5.0,
            100.0,
            vec![subscription("sr", "receiver", 10.0, SubscriptionType::Sr, 1)],
            vec![],
        );

        let evaluation = credit.evaluate();

        assert!(evaluation.booked_subscriptions().is_empty());
        assert_eq!(evaluation.unbookable_sr_transfers().len(), 1);
        assert_eq!(evaluation.unbookable_sr_transfers()[0].receiver(), "receiver");
        assert_eq!(evaluation.unbookable_sr_transfers()[0].amount(), 10.0);
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
