pub type Result<T> = core::result::Result<T, Error>;

#[derive(derive_more::Display, derive_more::Error, Debug)]
pub enum Error {
    #[display("Not found: {_0}")]
    NotFound(#[error(not(source))] &'static str),
    #[display("Validation failed: {_0}")]
    Validation(#[error(not(source))] &'static str),
    DbError(mongodb::error::Error),
    DbBsonDeError(mongodb::bson::de::Error),
    DbBsonSerError(mongodb::bson::ser::Error),
    DbObjectIdError(mongodb::bson::oid::Error),
}

impl From<mongodb::error::Error> for Error {
    fn from(value: mongodb::error::Error) -> Self {
        Self::DbError(value)
    }
}

impl From<mongodb::bson::de::Error> for Error {
    fn from(value: mongodb::bson::de::Error) -> Self {
        Self::DbBsonDeError(value)
    }
}

impl From<mongodb::bson::ser::Error> for Error {
    fn from(value: mongodb::bson::ser::Error) -> Self {
        Self::DbBsonSerError(value)
    }
}

impl From<mongodb::bson::oid::Error> for Error {
    fn from(value: mongodb::bson::oid::Error) -> Self {
        Self::DbObjectIdError(value)
    }
}
