use actix_web::{App, HttpServer, middleware::Logger, web};
use anyhow::{Context, Result};
use utoipa_actix_web::{AppExt, scope};
use utoipa_swagger_ui::SwaggerUi;

use crate::{config::Config, db::mongo_client::MongoClient, routes::{configure_routes, ApiDoc}};

mod app;
mod api;
mod config;
mod db;
mod domain;
mod routes;

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

    let uri = std::env::var("DB_URI")
        .unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
    let database = std::env::var("DB_DATABASE")
        .unwrap_or_else(|_| "credit_exchanger".to_string());

    // Initialize DB client
    let client = MongoClient::new(&uri, &database)
        .await
        .expect("Failed to initialize MongoDB client");

    HttpServer::new(move || {
        let logger = Logger::default();
        let mongo_client = web::Data::new(client.clone());

        App::new()
            .into_utoipa_app()
            .openapi(app::openapi())
            .map(|app| app.wrap(logger))
            .app_data(mongo_client)
            .service(scope::scope("/api").configure(configure_routes))
            .openapi_service(|api| SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", api))
            .into_app()
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
