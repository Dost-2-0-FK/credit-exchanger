use serde::{Deserialize, Serialize};

use crate::domain::credit::Credit;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateCreditRequest {
    total: f32,
    last_day_average: f32,
    subscription_ids: Vec<String>,
    history: Vec<f32>,
}

impl CreateCreditRequest {
    pub(crate) fn total(&self) -> f32 {
        self.total
    }

    pub(crate) fn last_day_average(&self) -> f32 {
        self.last_day_average
    }

    pub(crate) fn subscription_ids(&self) -> &[String] {
        &self.subscription_ids
    }

    pub(crate) fn history(&self) -> &[f32] {
        &self.history
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetCreditResponse {
    id: String,
    total: f32,
    last_day_average: f32,
    subscription_ids: Vec<String>,
    history: Vec<f32>,
}

impl GetCreditResponse {
    pub(crate) fn from_credit(id: String, credit: Credit) -> Self {
        Self {
            id,
            total: credit.total(),
            last_day_average: credit.last_day_average(),
            subscription_ids: credit
                .subscriptions()
                .iter()
                .map(|subscription| subscription.id().to_string())
                .collect(),
            history: credit.history().to_vec(),
        }
    }

    pub(crate) fn from_create_request(id: String, request: &CreateCreditRequest) -> Self {
        Self {
            id,
            total: request.total(),
            last_day_average: request.last_day_average(),
            subscription_ids: request.subscription_ids().to_vec(),
            history: request.history().to_vec(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreditBalanceResponse {
    pub credit_type: String,
    pub balance: f32,
    pub hourly: f32, // most recent hourly income from history
}

impl CreditBalanceResponse {
    pub(crate) fn from_credit(credit_type: String, credit: &Credit) -> Self {
        let hourly = credit
            .history()
            .last()
            .copied()
            .unwrap_or(0.0);
        Self {
            credit_type,
            balance: credit.total(),
            hourly,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListCreditsResponse {
    pub credits: Vec<CreditBalanceResponse>,
}