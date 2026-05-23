use std::sync::Arc;

use derive_more::Constructor;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::{
    api::user::User as ApiUser,
    db::{
        base_user::BaseUser, bloc_user::BlocUser, error::Result, individual_user::IndividualUser,
        mongo_client::MongoClient, unit_user::UnitUser, zone_user::ZoneUser,
    },
    domain,
};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum User {
    Bloc(BaseUser<BlocUser>),
    Zone(BaseUser<ZoneUser>),
    Individual(BaseUser<IndividualUser>),
    Unit(BaseUser<UnitUser>),
}

impl User {
    pub(crate) fn from_api_user(db_id: ObjectId, user: ApiUser) -> Self {
        match user {
            ApiUser::Bloc(user) => Self::Bloc(BaseUser::new(
                db_id,
                user.id().to_string(),
                BlocUser::from_domain(user.role()),
                user.credit().as_ref().clone(),
            )),
            ApiUser::Zone(user) => Self::Zone(BaseUser::new(
                db_id,
                user.id().to_string(),
                ZoneUser::from_domain(user.role()),
                user.credit().as_ref().clone(),
            )),
            ApiUser::Individual(user) => Self::Individual(BaseUser::new(
                db_id,
                user.id().to_string(),
                IndividualUser::from_domain(user.role()),
                user.credit().as_ref().clone(),
            )),
            ApiUser::Unit(user) => Self::Unit(BaseUser::new(
                db_id,
                user.id().to_string(),
                UnitUser::from_domain(user.role()),
                user.credit().as_ref().clone(),
            )),
        }
    }

    pub(crate) fn into_api_user(self) -> ApiUser {
        match self {
            Self::Bloc(user) => ApiUser::Bloc(domain::base_user::BaseUser::<
                domain::bloc_user::BlocUser,
            >::new(
                user.role.resources,
                &user.id,
                Arc::new(user.credit),
            )),
            Self::Zone(user) => ApiUser::Zone(domain::base_user::BaseUser::<
                domain::zone_user::ZoneUser,
            >::new(
                user.role.resources,
                &user.id,
                Arc::new(user.credit),
            )),
            Self::Individual(user) => {
                ApiUser::Individual(domain::base_user::BaseUser::<
                    domain::individual_user::IndividualUser,
                >::new(&user.id, Arc::new(user.credit)))
            }
            Self::Unit(user) => ApiUser::Unit(domain::base_user::BaseUser::<
                domain::unit_user::UnitUser,
            >::new(&user.id, Arc::new(user.credit))),
        }
    }

    pub(crate) fn is_unit(&self) -> bool {
        matches!(self, Self::Unit(_))
    }
}

#[derive(Constructor)]
pub(crate) struct UsersRepository {
    db: MongoClient,
}

impl UsersRepository {
    pub(crate) async fn get_user(&self, user_id: &str) -> Result<Option<ApiUser>> {
        let user_doc = self
            .db
            .get_document_by_field("users", "id", user_id)
            .await?;
        let Some(user_doc) = user_doc else {
            return Ok(None);
        };

        let user = mongodb::bson::from_document::<User>(user_doc)?;
        Ok(Some(user.into_api_user()))
    }

    pub(crate) async fn get_db_user(&self, user_id: &str) -> Result<Option<User>> {
        let user_doc = self
            .db
            .get_document_by_field("users", "id", user_id)
            .await?;
        let Some(user_doc) = user_doc else {
            return Ok(None);
        };

        let user = mongodb::bson::from_document::<User>(user_doc)?;
        Ok(Some(user))
    }

    pub(crate) async fn list_users(&self) -> Result<Vec<ApiUser>> {
        let user_docs = self.db.list_documents("users").await?;
        user_docs
            .into_iter()
            .map(|user_doc| {
                mongodb::bson::from_document::<User>(user_doc)
                    .map(User::into_api_user)
                    .map_err(Into::into)
            })
            .collect()
    }

    pub(crate) async fn insert_user(&self, user: ApiUser) -> Result<ApiUser> {
        let db_user = User::from_api_user(ObjectId::new(), user);
        let doc = mongodb::bson::to_document(&db_user)?;
        self.db.insert_document("users", doc).await?;
        Ok(db_user.into_api_user())
    }

    pub(crate) async fn update_unit_last_day_average(
        &self,
        user_id: &str,
        last_day_average: f32,
    ) -> Result<Option<ApiUser>> {
        let update = mongodb::bson::doc! {
            "$set": {
                "credit.last_day_average": last_day_average,
            }
        };

        self.db
            .update_document_by_field("users", "id", user_id, update)
            .await?;
        self.get_user(user_id).await
    }
}
