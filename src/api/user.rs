use serde::Serialize;

use crate::domain::{base_user::BaseUser, bloc_user::BlocUser, individual_user::IndividualUser, unit_user::UnitUser, zone_user::ZoneUser};

#[derive(Serialize)]
#[serde(tag = "type")] // Adds a "type" field to the serialized JSON
pub(crate) enum User {
    Bloc(BaseUser<BlocUser>),
    Zone(BaseUser<ZoneUser>),
    Individual(BaseUser<IndividualUser>),
    Unit(BaseUser<UnitUser>),
}
