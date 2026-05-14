use std::sync::Arc;

use serde::Serialize;

use crate::domain::{base_user::BaseUser, credit::Credit};

#[derive(Serialize)]
pub(crate) struct UnitUser {}

impl BaseUser<UnitUser> {
    pub(crate) fn new(id: &str, credit: Arc<Credit>) -> Self {
        BaseUser::new_base_user(UnitUser {}, id.into(), credit)
    }
}
