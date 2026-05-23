use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct UnitUser {}

impl UnitUser {
    pub(crate) fn from_domain(_user: &crate::domain::unit_user::UnitUser) -> Self {
        Self {}
    }
}
