use actix_web::{App, HttpServer, middleware::Logger, web};
use anyhow::{Context, Result};

use crate::{config::Config, service::{echo, hello, get_user, get_users, create_user, update_user}};

mod service;
mod config;
mod domain;
mod api;

async fn setup() -> Result<()> {
    env_logger::init();
    let config_file_name = "credit-exchanger.toml";
    let _c = Config::parse(config_file_name)
        .await
        .context(format!("parsing config file: {config_file_name}"))?;
    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    setup().await.map_err(|e| {
            log::error!("{:#}", e);
            std::io::Error::other(e)
        })?;

    HttpServer::new(|| {
        let logger = Logger::default();

        App::new().wrap(logger).service(
            web::scope("/api")
                .service(hello)
                .service(echo)
                .service(get_user)
                .service(get_users)
                .service(create_user)
                .service(update_user),
        )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
