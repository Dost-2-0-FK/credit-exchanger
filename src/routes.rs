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
        user::{CreateUserRequest, PatchUserRequest, User, UserType},
    },
    db::{
        credit::CreditRepository, mongo_client::MongoClient, subscription::SubscriptionsRepository,
        user::UsersRepository,
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
async fn get_user(client: web::Data<MongoClient>, user_id: Path<String>) -> impl Responder {
    let user_id = user_id.into_inner();
    let repository = UsersRepository::new(client.get_ref().clone());

    match repository.get_user(&user_id).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => HttpResponse::InternalServerError().body(format!("Failed to get user: {err}")),
    }
}

#[get("/users")]
// `GET /api/users`
async fn get_users(client: web::Data<MongoClient>) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());

    match repository.list_users().await {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to list users: {err}"))
        }
    }
}

#[post("/users")]
// `POST /api/users`
async fn create_user(
    client: web::Data<MongoClient>,
    body: web::Json<CreateUserRequest>,
) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());
    let body = body.into_inner();
    let user_id = body.id();
    let credit = Arc::new(Credit::new(0.0, 0.0, vec![], vec![]));
    let resources: HashMap<String, Credit> = HashMap::new();

    let user = match body.user_type() {
        UserType::Bloc => User::Bloc(BaseUser::<BlocUser>::new(resources, user_id, credit)),
        UserType::Zone => User::Zone(
            domain::base_user::BaseUser::<domain::zone_user::ZoneUser>::new(
                resources, user_id, credit,
            ),
        ),
        UserType::Individual => User::Individual(domain::base_user::BaseUser::<
            domain::individual_user::IndividualUser,
        >::new(user_id, credit)),
        UserType::Unit => User::Unit(
            domain::base_user::BaseUser::<domain::unit_user::UnitUser>::new(user_id, credit),
        ),
    };

    match repository.insert_user(user).await {
        Ok(user) => HttpResponse::Created().json(user),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to create user: {err}"))
        }
    }
}

#[patch("/users/{user_id}")]
// `PATCH /api/users/{user_id}`
async fn update_user(
    client: web::Data<MongoClient>,
    user_id: Path<String>,
    body: web::Json<PatchUserRequest>,
) -> impl Responder {
    let repository = UsersRepository::new(client.get_ref().clone());
    let user_id = user_id.into_inner();
    let body = body.into_inner();

    let user = match repository.get_db_user(&user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().finish(),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Failed to get user: {err}"));
        }
    };

    if !user.is_unit() {
        return HttpResponse::MethodNotAllowed().body("Only unit users can be updated");
    }

    if body.credit_type() != "money" {
        return HttpResponse::NotFound().body("Credit type not found for unit user");
    }

    match repository
        .update_unit_last_day_average(&user_id, body.last_day_average())
        .await
    {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to update user: {err}"))
        }
    }
}

// ### CREDITS

#[get("/credits/{credit_id}")]
// `GET /api/credits/{credit_id}`
async fn get_credit(client: web::Data<MongoClient>, credit_id: Path<String>) -> impl Responder {
    let repository = CreditRepository::new(
        client.get_ref().clone(),
        SubscriptionsRepository::new(client.get_ref().clone()),
    );
    let credit_id = credit_id.into_inner();

    match repository.get_credit(&credit_id).await {
        Ok(Some(credit)) => {
            HttpResponse::Ok().json(GetCreditResponse::from_credit(credit_id, credit))
        }
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(crate::db::error::Error::DbObjectIdError(_)) => {
            HttpResponse::BadRequest().body("Invalid credit id format")
        }
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to get credit: {err}"))
        }
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

// ### SUBSCRIPTIONS

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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test};
    use std::env;

    const TEST_DB_URI_ENV: &str = "TEST_MONGODB_URI";
    const TEST_DB_PREFIX: &str = "credit_exchanger_test";

    fn test_db_uri() -> String {
        env::var(TEST_DB_URI_ENV).unwrap_or_else(|_| "mongodb://localhost:27017".to_string())
    }

    async fn test_client() -> web::Data<crate::db::mongo_client::MongoClient> {
        let db_name = format!(
            "{}_{}",
            TEST_DB_PREFIX,
            mongodb::bson::oid::ObjectId::new().to_hex()
        );
        let client = crate::db::mongo_client::MongoClient::new(&test_db_uri(), &db_name)
            .await
            .expect("Failed to connect to test MongoDB");

        web::Data::new(client)
    }

    // ── Users ─────────────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_get_user_returns_user() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(get_user),
        )
        .await;

        let create_req_1 = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "alice", "userType": "individual"}))
            .to_request();
        test::call_service(&app, create_req_1).await;

        let create_req_2 = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "bob", "userType": "unit"}))
            .to_request();
        test::call_service(&app, create_req_2).await;

        let get_req = test::TestRequest::get().uri("/users/alice").to_request();
        let resp = test::call_service(&app, get_req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["id"], "alice");
        assert_eq!(body["userType"], "individual")
    }

    #[actix_web::test]
    async fn test_get_user_not_found() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(get_user)).await;
        let req = test::TestRequest::get()
            .uri("/users/no_such_user_xyz")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_get_users_returns_empty() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(get_users)).await;
        let req = test::TestRequest::get().uri("/users").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body, serde_json::json!([]));
    }

    #[actix_web::test]
    async fn test_get_users_returns_all() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(get_users),
        )
        .await;

        let bloc_id = format!("bloc_{}", mongodb::bson::oid::ObjectId::new().to_hex());
        let zone_id = format!("zone_{}", mongodb::bson::oid::ObjectId::new().to_hex());
        let individual_id = format!(
            "individual_{}",
            mongodb::bson::oid::ObjectId::new().to_hex()
        );
        let unit_id = format!("unit_{}", mongodb::bson::oid::ObjectId::new().to_hex());

        for (id, user_type) in [
            (&bloc_id, "bloc"),
            (&zone_id, "zone"),
            (&individual_id, "individual"),
            (&unit_id, "unit"),
        ] {
            let req = test::TestRequest::post()
                .uri("/users")
                .set_json(serde_json::json!({ "id": id, "userType": user_type }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);
        }

        let req = test::TestRequest::get().uri("/users").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let users: serde_json::Value = test::read_body_json(resp).await;
        let user_array = users.as_array().expect("users response should be an array");

        let returned_ids: std::collections::HashSet<String> = user_array
            .iter()
            .filter_map(|user| {
                user.get("id")
                    .and_then(serde_json::Value::as_str)
                    .map(ToString::to_string)
            })
            .collect();

        assert!(returned_ids.contains(&bloc_id));
        assert!(returned_ids.contains(&zone_id));
        assert!(returned_ids.contains(&individual_id));
        assert!(returned_ids.contains(&unit_id));
    }

    #[actix_web::test]
    async fn test_create_user_bloc() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_user)).await;
        let req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "test_bloc_user", "userType": "bloc"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["id"], "test_bloc_user");
        assert_eq!(body["userType"], "bloc");
        assert_eq!(body["resources"], serde_json::json!({}));
        assert_eq!(body["credit"]["total"], 0.0);
        assert_eq!(body["credit"]["last_day_average"], 0.0);
        assert_eq!(body["credit"]["subscriptions"], serde_json::json!([]));
        assert_eq!(body["credit"]["history"], serde_json::json!([]));
    }

    #[actix_web::test]
    async fn test_create_user_zone() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_user)).await;
        let req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "test_zone_user", "userType": "zone"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["id"], "test_zone_user");
        assert_eq!(body["userType"], "zone");
        assert_eq!(body["resources"], serde_json::json!({}));
        assert_eq!(body["credit"]["total"], 0.0);
        assert_eq!(body["credit"]["last_day_average"], 0.0);
        assert_eq!(body["credit"]["subscriptions"], serde_json::json!([]));
        assert_eq!(body["credit"]["history"], serde_json::json!([]));
    }

    #[actix_web::test]
    async fn test_create_user_individual() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_user)).await;
        let req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "test_individual_user", "userType": "individual"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["id"], "test_individual_user");
        assert_eq!(body["userType"], "individual");
        assert_eq!(body["credit"]["total"], 0.0);
        assert_eq!(body["credit"]["last_day_average"], 0.0);
        assert_eq!(body["credit"]["subscriptions"], serde_json::json!([]));
        assert_eq!(body["credit"]["history"], serde_json::json!([]));
    }

    #[actix_web::test]
    async fn test_create_user_unit() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_user)).await;
        let req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "test_unit_user", "userType": "unit"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["id"], "test_unit_user");
        assert_eq!(body["userType"], "unit");
        assert_eq!(body["credit"]["total"], 0.0);
        assert_eq!(body["credit"]["last_day_average"], 0.0);
        assert_eq!(body["credit"]["subscriptions"], serde_json::json!([]));
        assert_eq!(body["credit"]["history"], serde_json::json!([]));
    }

    #[actix_web::test]
    async fn test_create_user_unknown_type_returns_bad_request() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_user)).await;
        let req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "bad_user", "userType": "unknownType"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_update_user_not_found() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(update_user)).await;
        let req = test::TestRequest::patch()
            .uri("/users/no_such_user_xyz")
            .set_json(serde_json::json!({"creditType": "money", "lastDayAverage": 10.0}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_update_non_unit_user_returns_method_not_allowed() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(update_user),
        )
        .await;

        // Create a bloc user first
        let create_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "patch_bloc_user", "userType": "bloc"}))
            .to_request();
        test::call_service(&app, create_req).await;

        let patch_req = test::TestRequest::patch()
            .uri("/users/patch_bloc_user")
            .set_json(serde_json::json!({"creditType": "money", "lastDayAverage": 10.0}))
            .to_request();
        let resp = test::call_service(&app, patch_req).await;
        assert_eq!(
            resp.status(),
            actix_web::http::StatusCode::METHOD_NOT_ALLOWED
        );
    }

    #[actix_web::test]
    async fn test_update_unit_user_credit_type_not_money_returns_not_found() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(update_user),
        )
        .await;

        let create_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "patch_unit_user_bad_credit", "userType": "unit"}))
            .to_request();
        test::call_service(&app, create_req).await;

        let patch_req = test::TestRequest::patch()
            .uri("/users/patch_unit_user_bad_credit")
            .set_json(serde_json::json!({"creditType": "unknown", "lastDayAverage": 10.0}))
            .to_request();
        let resp = test::call_service(&app, patch_req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_update_unit_user_returns_ok() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_user)
                .service(update_user),
        )
        .await;

        let create_req = test::TestRequest::post()
            .uri("/users")
            .set_json(serde_json::json!({"id": "patch_unit_user", "userType": "unit"}))
            .to_request();
        test::call_service(&app, create_req).await;

        let patch_req = test::TestRequest::patch()
            .uri("/users/patch_unit_user")
            .set_json(serde_json::json!({"creditType": "money", "lastDayAverage": 42.0}))
            .to_request();
        let resp = test::call_service(&app, patch_req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
    }

    // ── Credits ───────────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_get_credit_invalid_id_returns_bad_request() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(get_credit)).await;
        let req = test::TestRequest::get()
            .uri("/credits/not_an_object_id")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_get_credit_not_found() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(get_credit)).await;
        let req = test::TestRequest::get()
            .uri("/credits/000000000000000000000001")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_create_credit_no_subscriptions() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_credit)).await;
        let req = test::TestRequest::post()
            .uri("/credits")
            .set_json(serde_json::json!({
                "total": 100.0,
                "lastDayAverage": 50.0,
                "subscriptionIds": [],
                "history": [10.0, 20.0]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);
    }

    #[actix_web::test]
    async fn test_create_credit_invalid_subscription_id_returns_bad_request() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_credit)).await;
        let req = test::TestRequest::post()
            .uri("/credits")
            .set_json(serde_json::json!({
                "total": 100.0,
                "lastDayAverage": 50.0,
                "subscriptionIds": ["not_a_valid_object_id"],
                "history": []
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_create_credit_nonexistent_subscription_returns_not_found() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(create_credit)).await;
        let req = test::TestRequest::post()
            .uri("/credits")
            .set_json(serde_json::json!({
                "total": 100.0,
                "lastDayAverage": 50.0,
                "subscriptionIds": ["000000000000000000000001"],
                "history": []
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    // ── Subscriptions ─────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_get_subscription_invalid_id_returns_bad_request() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(get_subscription)).await;
        let req = test::TestRequest::get()
            .uri("/subscriptions/not_an_object_id")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_get_subscription_not_found() {
        let client = test_client().await;
        let app = test::init_service(App::new().app_data(client).service(get_subscription)).await;
        let req = test::TestRequest::get()
            .uri("/subscriptions/000000000000000000000001")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_create_subscription_returns_created() {
        let client = test_client().await;
        let app =
            test::init_service(App::new().app_data(client).service(create_subscription)).await;
        let req = test::TestRequest::post()
            .uri("/subscriptions")
            .set_json(serde_json::json!({
                "receiver": 42,
                "value": 10.5,
                "subscriptionType": "sr",
                "priority": 1
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::CREATED);
    }

    #[actix_web::test]
    async fn test_create_and_get_subscription() {
        let client = test_client().await;
        let app = test::init_service(
            App::new()
                .app_data(client)
                .service(create_subscription)
                .service(get_subscription),
        )
        .await;

        // Create
        let create_req = test::TestRequest::post()
            .uri("/subscriptions")
            .set_json(serde_json::json!({
                "receiver": 99,
                "value": 5.0,
                "subscriptionType": "contract",
                "priority": 2
            }))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        assert_eq!(create_resp.status(), actix_web::http::StatusCode::CREATED);

        let body: serde_json::Value = test::read_body_json(create_resp).await;
        let id = body["id"].as_str().expect("id missing in response");

        // Get by returned id
        let get_req = test::TestRequest::get()
            .uri(&format!("/subscriptions/{id}"))
            .to_request();
        let get_resp = test::call_service(&app, get_req).await;
        assert_eq!(get_resp.status(), actix_web::http::StatusCode::OK);
    }
}
