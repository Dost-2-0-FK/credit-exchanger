use actix_web::{HttpResponse, Responder, get, post, web};
use serde::Deserialize;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}

#[derive(Deserialize, Debug)]
struct CreditBooking {
    id: String,
    receiver: String,
    // TODO check whether value is always positive and not a float
    value: u32,
}

#[post("/credits/book")]
/// `POST /api/credits/book?id=<unique_id>&receiver=<receiver_id>&value=<value>`
async fn post_credit_booking(credit_booking: web::Query<CreditBooking>) -> impl Responder {
    let _credit_booking = credit_booking.into_inner();
    HttpResponse::Ok()
}

#[derive(Deserialize, Debug)]
enum ResourceBooking {
    WithResource {
        id: String,
        receiver: String,
        resource: String,
        // TODO check whether value is always positive and not a float
        value: u32,
    },
    WithoutResource {
        id: String,
        receiver: String,
        // TODO should vec len >= 1?
        value: Vec<u32>,
    },
}

#[post("/resource/book")]
/// `POST /api/resource/book?id=<unique_id>&receiver=<receiver_id>&resource=<resource>&value=<value>`
async fn post_resource_booking(resource_booking: web::Query<ResourceBooking>) -> impl Responder {
    let _resource_booking = resource_booking.into_inner();
    HttpResponse::Ok()
}
