use utoipa::OpenApi;

pub(crate) fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

#[derive(OpenApi)]
#[openapi(
        tags(
            (name = "users", description = "Endpoints related to users."),
            (name = "credits", description = "Endpoints related to credits."),
            (name = "subscriptions", description = "Endpoints related to subscriptions."),
            (name = "evaluations", description = "Endpoints related to evaluations."),
        ),
    )]
struct ApiDoc;
