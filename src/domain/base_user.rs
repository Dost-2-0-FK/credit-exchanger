use std::sync::Arc;

use serde::Serialize;

use crate::domain::credit::Credit;

#[derive(Serialize)]
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
}
