mod error;
mod string_normalization;

use std::{path::Path};

use serde::Deserialize;
use tokio::fs;

use self::{
    error::{Error, Result},
    string_normalization::{VecLowercaseString},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    resources: VecLowercaseString,
}

impl Config {
    pub(crate) async fn parse(path: impl AsRef<Path>) -> Result<Self> {
        let config = fs::read_to_string(path).await.map_err(Error::Io)?;
        Self::parse_from_str(&config)
    }

    fn parse_from_str(config: &str) -> Result<Self> {
        let config = toml::from_str::<Config>(config).map_err(Error::Toml)?;

        Ok(config)
    }
}
