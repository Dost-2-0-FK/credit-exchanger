use std::{collections::HashMap, sync::Arc};

use serde::Serialize;

use crate::domain::{base_user::BaseUser, credit::Credit};

#[derive(Serialize)]
pub(crate) struct BlocUser {
    resources: HashMap<String, Credit>,
}

impl BaseUser<BlocUser> {
    pub(crate) fn new(resources: HashMap<String, Credit>, id: &str, credit: Arc<Credit>) -> Self {
        BaseUser::new_base_user(BlocUser { resources }, id.into(), credit)
    }
}
