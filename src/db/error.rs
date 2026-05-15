pub type Result<T> = core::result::Result<T, Error>;

#[derive(derive_more::Display, derive_more::Error, Debug, derive_more::From)]
pub enum Error {
    #[display("Not found: {_0}")]
    NotFound(#[error(not(source))] &'static str),
    DbError(#[from] mongodb::error::Error),
    DbBsonError(#[from] mongodb::bson::de::Error),
}

