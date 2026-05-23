use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::domain::{base_user::BaseUser, credit::Credit};

#[derive(Serialize, Deserialize)]
pub(crate) struct ZoneUser {
    resources: HashMap<String, Credit>,
}

impl BaseUser<ZoneUser> {
    pub(crate) fn new(resources: HashMap<String, Credit>, id: &str, credit: Arc<Credit>) -> Self {
        BaseUser::new_base_user(
            ZoneUser {
                resources: resources.into(),
            },
            id.into(),
            credit,
        )
    }
}

impl ZoneUser {
    pub(crate) fn resources(&self) -> &HashMap<String, Credit> {
        &self.resources
    }
}
