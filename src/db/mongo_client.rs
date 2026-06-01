use crate::db::error::Result;
use mongodb::bson::Bson;
use mongodb::bson::oid::ObjectId;
use mongodb::{Client, options::ClientOptions};

#[derive(Clone)]
pub struct MongoClient {
    client: Client,
    database_name: String,
}

impl MongoClient {
    pub(crate) async fn new(uri: &str, database_name: &str) -> Result<Self> {
        let options = ClientOptions::parse(uri).await?;
        let client = Client::with_options(options)?;
        Ok(Self {
            client,
            database_name: database_name.to_string(),
        })
    }

    #[allow(dead_code)]
    pub(crate) fn database_name(&self) -> &str {
        &self.database_name
    }

    pub(crate) async fn get_document_by_object_id(
        &self,
        collection: &str,
        id: &ObjectId,
    ) -> Result<Option<mongodb::bson::Document>> {
        let db = self.client.database(&self.database_name);
        let coll = db.collection(collection);
        let filter = mongodb::bson::doc! { "_id": id };
        let document = coll.find_one(filter, None).await?;
        Ok(document)
    }

    pub(crate) async fn get_document_by_field(
        &self,
        collection: &str,
        field: &str,
        value: impl Into<Bson>,
    ) -> Result<Option<mongodb::bson::Document>> {
        let db = self.client.database(&self.database_name);
        let coll = db.collection::<mongodb::bson::Document>(collection);
        let filter = mongodb::bson::doc! { field: value.into() };
        let document = coll.find_one(filter, None).await?;
        Ok(document)
    }

    pub(crate) async fn list_documents(
        &self,
        collection: &str,
    ) -> Result<Vec<mongodb::bson::Document>> {
        let db = self.client.database(&self.database_name);
        let coll = db.collection::<mongodb::bson::Document>(collection);
        let mut cursor = coll.find(None, None).await?;
        let mut documents = Vec::new();

        while cursor.advance().await? {
            documents.push(cursor.deserialize_current()?);
        }

        Ok(documents)
    }

    pub(crate) async fn update_document_by_field(
        &self,
        collection: &str,
        field: &str,
        value: impl Into<Bson>,
        update: mongodb::bson::Document,
    ) -> Result<()> {
        let db = self.client.database(&self.database_name);
        let coll = db.collection::<mongodb::bson::Document>(collection);
        let filter = mongodb::bson::doc! { field: value.into() };
        coll.update_one(filter, update, None).await?;
        Ok(())
    }

    pub(crate) async fn replace_document_by_field(
        &self,
        collection: &str,
        field: &str,
        value: impl Into<Bson>,
        document: mongodb::bson::Document,
    ) -> Result<()> {
        let db = self.client.database(&self.database_name);
        let coll = db.collection::<mongodb::bson::Document>(collection);
        let filter = mongodb::bson::doc! { field: value.into() };
        coll.replace_one(filter, document, None).await?;
        Ok(())
    }

    pub(crate) async fn insert_document(
        &self,
        collection: &str,
        document: impl Into<mongodb::bson::Document>,
    ) -> Result<()> {
        let db = self.client.database(&self.database_name);
        let coll = db.collection(collection);
        coll.insert_one(document.into(), None).await?;
        Ok(())
    }
}
