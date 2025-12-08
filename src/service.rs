use actix_web::{Either, HttpResponse, Responder, get, post};
use actix_web_lab::extract::Query;
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
async fn post_credit_booking(credit_booking: Query<CreditBooking>) -> impl Responder {
    let _credit_booking = credit_booking.into_inner();
    HttpResponse::Ok()
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct SingleResourceBooking {
    id: String,
    receiver: String,
    resource: String,
    // TODO check whether value is always positive and not a float
    value: u32,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct AllResourcesBooking {
    id: String,
    receiver: String,
    #[serde(rename = "value")]
    values: Vec<u32>,
}

#[post("/resource/book")]
/// `POST /api/resource/book?id=<unique_id>&receiver=<receiver_id>&resource=<resource>&value=<value>`
async fn post_resource_booking(
    resource_booking: Either<Query<SingleResourceBooking>, Query<AllResourcesBooking>>,
) -> impl Responder {
    match resource_booking {
        Either::Left(single) => {
            // handle single resource booking
            log::debug!("got single: {single:?}");
        }
        Either::Right(all) => {
            // handle all resource booking
            log::debug!("got all: {all:?}");
        }
    }

    HttpResponse::Ok()
}

// TODO: curl -vv -X POST "http://127.0.0.1:8080/api/resource/book?id=book-id-123&receiver=rec-id-123&value=1&resource=resrouce-123&value=2"
// does not throw an error, AllResourcesBooking is matched and resources param is ignored. This should throw an error instead
// maybe it makes sense to use a request body here instead of query params
// https://medium.com/@arijitbasubd/understanding-http-methods-when-to-use-url-params-query-params-and-request-body-9ca94baefbf7