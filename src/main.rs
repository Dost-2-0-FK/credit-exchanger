use actix_web::{App, HttpServer, middleware::Logger, web};

use crate::service::{echo, hello, post_credit_booking, post_resource_booking};

mod service;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

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
