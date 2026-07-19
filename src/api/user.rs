use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::{
    base_user::BaseUser,
    bloc_user::BlocUser,
    credit::ListUserCredit,
    individual_user::IndividualUser,
    unit_user::UnitUser,
    zone_user::ZoneUser,
};

/// This struct contains a user with reduces fields
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all="camelCase")]
pub(crate) struct ListUser {
    id: String,
    user_type: UserType,
    credit: ListUserCredit,
    #[serde(skip_serializing_if = "Option::is_none")]
    resources: Option<HashMap<String, ListUserCredit>>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(tag = "userType")]
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

impl User {
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Bloc(user) => user.id(),
            Self::Zone(user) => user.id(),
            Self::Individual(user) => user.id(),
            Self::Unit(user) => user.id(),
        }
    }
}

impl From<User> for ListUser {
    fn from(value: User) -> Self {
        let (id, user_type, credit, resources) = match value {
            User::Bloc(base_user) => (
                base_user.id().into(),
                UserType::Bloc,
                base_user.credit().clone().into(),
                base_user.role().resources().clone().into(),
            ),
            User::Zone(base_user) => (
                base_user.id().into(),
                UserType::Zone,
                base_user.credit().clone().into(),
                base_user.role().resources().clone().into(),
            ),
            User::Individual(base_user) => (
                base_user.id().into(),
                UserType::Individual,
                base_user.credit().clone().into(),
                None,
            ),
            User::Unit(base_user) => (
                base_user.id().into(),
                UserType::Unit,
                base_user.credit().clone().into(),
                base_user.role().resources().clone().into(),
            ),
        };
        let resources = resources.map(|resources| {
            resources
                .into_iter()
                .map(|(key, value)| (key, value.into()))
                .collect()
        });
        Self {
            id,
            user_type,
            credit,
            resources,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) enum UserType {
    Bloc,
    Zone,
    Individual,
    Unit,
}
