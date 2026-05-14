use std::{collections::HashMap, sync::Arc};

use actix_web::{HttpResponse, Responder, get, patch, post, web::{Path}};
use serde::Deserialize;

use crate::{api::user::User, domain::{base_user::BaseUser, bloc_user::BlocUser, credit::Credit}};

// TODO: adjust those structs according to new api contract and put into dedicated module
#[derive(Deserialize, Debug)]
struct CreditBooking {
    id: String,
    receiver: String,
    // TODO check whether value is always positive and not a float
    value: u32,
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

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}

// ### USERS

#[get("/users/{user_id}")]
// `GET /api/users/{user_id}`
async fn get_user(_user_id: Path<String>) -> impl Responder {
    // TODO: implement
    let credit = Arc::new(Credit::new(
        1.1,
        2.2,
        vec![],
        vec![],
    ));
    let resources: HashMap<String, Credit> = HashMap::new();

    let user = User::Bloc(BaseUser::<BlocUser>::new(resources, "xxx", credit));
    HttpResponse::Ok().json(user)
}

#[get("/users")]
// `GET /api/users`    
async fn get_users() -> impl Responder {
    // TODO: implement
    HttpResponse::Ok()
}

#[post("/users")]
// `POST /api/users`
async fn create_user() -> impl Responder {
    // TODO: implement
    HttpResponse::Ok()
}

#[patch("/users/{user_id}")]
// `PATCH /api/users/{user_id}`
async fn update_user() -> impl Responder {
    // TODO: implement
    HttpResponse::Ok()
}
