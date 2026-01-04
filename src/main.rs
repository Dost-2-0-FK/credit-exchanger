use actix_web::{App, HttpServer, middleware::Logger, web};
use anyhow::{Context, Result};

use crate::{config::Config, service::{echo, hello, post_credit_booking, post_resource_booking}};

mod service;
mod config;

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
                .service(post_credit_booking)
                .service(post_resource_booking),
        )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
