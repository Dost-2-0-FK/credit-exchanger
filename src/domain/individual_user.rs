use std::sync::Arc;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::{base_user::BaseUser, credit::Credit};

#[derive(Serialize, Deserialize, ToSchema)]
pub(crate) struct IndividualUser {}

impl BaseUser<IndividualUser> {
    pub(crate) fn new(id: &str, credit: Arc<Credit>) -> Self {
        BaseUser::new_base_user(IndividualUser {}, id.into(), credit)
    }
}
