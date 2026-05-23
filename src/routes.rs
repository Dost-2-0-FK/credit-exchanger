use std::{collections::HashMap, sync::Arc};

use actix_web::{
    HttpResponse, Responder, get, patch, post,
    web::{self, Path},
};
use mongodb::bson::doc;
use serde::Deserialize;

use crate::{
    api::{
        credit::{CreateCreditRequest, GetCreditResponse},
        subscription::{CreateSubscriptionRequest, GetSubscriptionResponse},
        user::User,
    },
    db::{
        credit::CreditRepository,
        mongo_client::MongoClient,
        subscription::SubscriptionsRepository,
    },
    domain::{self, base_user::BaseUser, bloc_user::BlocUser, credit::Credit},
};

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
    let credit = Arc::new(Credit::new(1.1, 2.2, vec![], vec![]));
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
async fn create_user(
    client: web::Data<MongoClient>,
    user: web::Json<serde_json::Value>,
) -> impl Responder {
    let document = doc! { "name": user["name"].as_str().unwrap_or("Unknown"), "age": user["age"].as_i64().unwrap_or(0) };

    match client.insert_document("users", document).await {
        Ok(_) => HttpResponse::Ok().body("User created successfully"),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to create user: {}", err))
        }
    }
}

#[patch("/users/{user_id}")]
// `PATCH /api/users/{user_id}`
async fn update_user() -> impl Responder {
    // TODO: implement
    HttpResponse::Ok()
}

// ### SUBSCRIPTIONS

#[get("/credits/{credit_id}")]
// `GET /api/credits/{credit_id}`
async fn get_credit(client: web::Data<MongoClient>, credit_id: Path<String>) -> impl Responder {
    let repository = CreditRepository::new(
        client.get_ref().clone(),
        SubscriptionsRepository::new(client.get_ref().clone()),
    );
    let credit_id = credit_id.into_inner();

    match repository.get_credit(&credit_id).await {
        Ok(Some(credit)) => HttpResponse::Ok().json(GetCreditResponse::from_credit(credit_id, credit)),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(crate::db::error::Error::DbObjectIdError(_)) => {
            HttpResponse::BadRequest().body("Invalid credit id format")
        }
        Err(err) => HttpResponse::InternalServerError().body(format!("Failed to get credit: {err}")),
    }
}

#[post("/credits")]
// `POST /api/credits`
async fn create_credit(
    client: web::Data<MongoClient>,
    body: web::Json<CreateCreditRequest>,
) -> impl Responder {
    let body = body.into_inner();
    let subscription_repository = SubscriptionsRepository::new(client.get_ref().clone());
    let credit_repository = CreditRepository::new(
        client.get_ref().clone(),
        SubscriptionsRepository::new(client.get_ref().clone()),
    );

    let mut domain_subscriptions = Vec::with_capacity(body.subscription_ids().len());
    for subscription_id in body.subscription_ids() {
        let object_id = match mongodb::bson::oid::ObjectId::parse_str(subscription_id) {
            Ok(object_id) => object_id,
            Err(_) => return HttpResponse::BadRequest().body("Invalid subscription id format"),
        };

        let subscription = match subscription_repository.get_subscription(&object_id).await {
            Ok(Some(subscription)) => subscription,
            Ok(None) => {
                return HttpResponse::NotFound().body("One or more subscriptions were not found");
            }
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Failed to resolve subscriptions: {err}"));
            }
        };

        domain_subscriptions.push(Arc::new(subscription));
    }

    let domain_credit = domain::credit::Credit::new(
        body.total(),
        body.last_day_average(),
        domain_subscriptions,
        body.history().to_vec(),
    );

    match credit_repository.insert_credit(domain_credit).await {
        Ok(id) => HttpResponse::Created().json(GetCreditResponse::from_create_request(id, &body)),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to create credit: {err}"))
        }
    }
}

#[get("/subscriptions/{subscription_id}")]
// `GET /api/subscriptions/{subscription_id}`
async fn get_subscription(
    client: web::Data<MongoClient>,
    subscription_id: Path<String>,
) -> impl Responder {
    let repository = SubscriptionsRepository::new(client.get_ref().clone());
    let subscription_id = subscription_id.into_inner();
    let object_id = match mongodb::bson::oid::ObjectId::parse_str(&subscription_id) {
        Ok(object_id) => object_id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid subscription id format"),
    };

    match repository.get_subscription(&object_id).await {
        Ok(Some(subscription)) => {
            HttpResponse::Ok().json(GetSubscriptionResponse::from(subscription))
        }
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to get subscription: {err}"))
        }
    }
}

#[post("/subscriptions")]
// `POST /api/subscriptions`
async fn create_subscription(
    client: web::Data<MongoClient>,
    body: web::Json<CreateSubscriptionRequest>,
) -> impl Responder {
    let repository = SubscriptionsRepository::new(client.get_ref().clone());
    let domain_subscription = domain::subscription::Subscription::new(
        mongodb::bson::oid::ObjectId::new().to_hex(),
        body.receiver(),
        body.value(),
        body.subscription_type(),
        body.priority(),
    );

    match repository.insert_subscription(domain_subscription).await {
        Ok(Some(subscription)) => {
            HttpResponse::Created().json(GetSubscriptionResponse::from(subscription))
        }
        Ok(None) => {
            HttpResponse::InternalServerError().body("Subscription creation returned no data")
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to create subscription: {err}")),
    }
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(hello)
        .service(echo)
        .service(get_user)
        .service(get_users)
        .service(create_user)
        .service(update_user)
        .service(get_credit)
        .service(create_credit)
        .service(get_subscription)
        .service(create_subscription);
}
