use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::domain::credit::Credit;

#[derive(Serialize, Deserialize)]
pub(crate) struct BaseUser<T> {
    #[serde(rename = "_id")]
    pub(crate) db_id: ObjectId,
    pub(crate) id: String,
    #[serde(flatten)]
    pub(crate) role: T,
    pub(crate) credit: Credit,
}

impl<T> BaseUser<T> {
    pub(crate) fn new(db_id: ObjectId, id: String, role: T, credit: Credit) -> Self {
        Self {
            db_id,
            id,
            role,
            credit,
        }
    }
}
