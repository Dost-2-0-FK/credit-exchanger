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
struct SingleResourceBooking {
    id: String,
    receiver: String,
    resource: String,
    // TODO check whether value is always positive and not a float
    value: u32,
}

#[derive(Deserialize, Debug)]
struct AllResourcesBooking {
    id: String,
    receiver: String,
    #[serde(rename = "value")]
    values: NonEmptyVec<u32>,
}

/// Enforce on deserialization that the inner vector must not be empty.
#[derive(Debug, derive_more::Deref)]
struct NonEmptyVec<T>(Vec<T>);

impl<'de, T> Deserialize<'de> for NonEmptyVec<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vector = Vec::<T>::deserialize(deserializer)?;

        if vector.is_empty() {
            return Err(serde::de::Error::custom(
                "must provide at least one element",
            ));
        }

        Ok(Self(vector))
    }
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
