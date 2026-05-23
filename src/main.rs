use actix_web::{App, HttpServer, middleware::Logger, web};
use anyhow::{Context, Result};

use crate::{config::Config, db::mongo_client::MongoClient, routes::configure_routes};

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

    let uri = "mongodb://localhost:27017";
    let database = "test_db";

    // Initialize MongoDB client
    let client = MongoClient::new(uri, database)
        .await
        .expect("Failed to initialize MongoDB client");

    // // Example: Insert a document
    // let document = doc! { "name": "John Doe", "age": 30 };
    // client.insert_document("users", document).await.expect("Failed to insert document");

    HttpServer::new(move || {
        let logger = Logger::default();
        let mongo_client = web::Data::new(client.clone());

        App::new()
            .wrap(logger)
            .app_data(mongo_client)
            .configure(configure_routes)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
