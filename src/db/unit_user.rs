use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::credit::Credit;

#[derive(Serialize, Deserialize)]
pub(crate) struct UnitUser {
    #[serde(default)]
    pub(crate) resources: HashMap<String, Credit>,
}

impl UnitUser {
    pub(crate) fn from_domain(user: &crate::domain::unit_user::UnitUser) -> Self {
        Self {
            resources: user.resources().clone(),
        }
    }
}
