use std::{collections::HashMap, sync::Arc};

use actix_web::{HttpResponse, Responder, get, patch, post, web::{self, Path}};
use mongodb::bson::doc;
use serde::Deserialize;

use crate::{api::user::User, mongo_client::MongoClient, domain::{base_user::BaseUser, bloc_user::BlocUser, credit::Credit}};

// TODO: adjust those structs according to new api contract and put into dedicated module
#[derive(Deserialize, Debug)]
struct _CreditBooking {
    id: String,
    receiver: String,
    // TODO check whether value is always positive and not a float
    value: u32,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct _SingleResourceBooking {
    id: String,
    receiver: String,
    resource: String,
    // TODO check whether value is always positive and not a float
    value: u32,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct _AllResourcesBooking {
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
async fn create_user(client: web::Data<MongoClient>, user: web::Json<serde_json::Value>) -> impl Responder {
    let document = doc! { "name": user["name"].as_str().unwrap_or("Unknown"), "age": user["age"].as_i64().unwrap_or(0) };

    match client.insert_document("users", document).await {
        Ok(_) => HttpResponse::Ok().body("User created successfully"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Failed to create user: {}", err)),
    }
}

#[patch("/users/{user_id}")]
// `PATCH /api/users/{user_id}`
async fn update_user() -> impl Responder {
    // TODO: implement
    HttpResponse::Ok()
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(hello)
       .service(echo)
       .service(get_user)
       .service(get_users)
       .service(create_user)
       .service(update_user);
}
