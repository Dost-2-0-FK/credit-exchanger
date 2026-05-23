use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::domain::credit::Credit;

#[derive(Serialize, Deserialize)]
pub(crate) struct BaseUser<T> {
    #[serde(flatten)]
    role: T,
    id: String,
    credit: Arc<Credit>,
}

impl<T> BaseUser<T> {
    pub(crate) fn new_base_user(role: T, id: &str, credit: Arc<Credit>) -> Self {
        Self {
            role,
            id: id.into(),
            credit,
        }
    }
}

impl<T> BaseUser<T> {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn role(&self) -> &T {
        &self.role
    }

    pub(crate) fn credit(&self) -> &Arc<Credit> {
        &self.credit
    }
}
