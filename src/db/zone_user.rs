use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::credit::Credit;

#[derive(Serialize, Deserialize)]
pub(crate) struct ZoneUser {
    pub(crate) resources: HashMap<String, Credit>,
}

impl ZoneUser {
    pub(crate) fn from_domain(user: &crate::domain::zone_user::ZoneUser) -> Self {
        Self {
            resources: user.resources().clone(),
        }
    }
}
