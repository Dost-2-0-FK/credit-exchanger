use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::{base_user::BaseUser, credit::Credit};

#[derive(Serialize, Deserialize, ToSchema)]
pub(crate) struct BlocUser {
    resources: HashMap<String, Credit>,
}

impl BaseUser<BlocUser> {
    pub(crate) fn new(resources: HashMap<String, Credit>, id: &str, credit: Arc<Credit>) -> Self {
        BaseUser::new_base_user(BlocUser { resources }, id.into(), credit)
    }
}

impl BlocUser {
    pub(crate) fn resources(&self) -> &HashMap<String, Credit> {
        &self.resources
    }
}
