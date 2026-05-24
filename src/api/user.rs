use serde::{Deserialize, Serialize};

use crate::domain::{
    base_user::BaseUser, bloc_user::BlocUser, individual_user::IndividualUser, unit_user::UnitUser,
    zone_user::ZoneUser,
};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum User {
    #[serde(rename = "bloc", alias = "Bloc")]
    Bloc(BaseUser<BlocUser>),
    #[serde(rename = "zone", alias = "Zone")]
    Zone(BaseUser<ZoneUser>),
    #[serde(rename = "individual", alias = "Individual")]
    Individual(BaseUser<IndividualUser>),
    #[serde(rename = "unit", alias = "Unit")]
    Unit(BaseUser<UnitUser>),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateUserRequest {
    id: String,
    user_type: UserType,
}

impl CreateUserRequest {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }
    
    pub(crate) fn user_type(&self) -> UserType {
        self.user_type
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PatchUserRequest {
    credit_type: String,
    last_day_average: f32,
}

impl PatchUserRequest {
    pub(crate) fn credit_type(&self) -> &str {
        &self.credit_type
    }

    pub(crate) fn last_day_average(&self) -> f32 {
        self.last_day_average
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub(crate) enum UserType {
    Bloc,
    Zone,
    Individual,
    Unit,
}
