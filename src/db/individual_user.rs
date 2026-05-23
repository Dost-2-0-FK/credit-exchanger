use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct IndividualUser {}

impl IndividualUser {
    pub(crate) fn from_domain(_user: &crate::domain::individual_user::IndividualUser) -> Self {
        Self {}
    }
}
