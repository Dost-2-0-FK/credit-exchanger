use mongodb::{Client, options::ClientOptions};
use crate::db::error::Result;

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

    pub(crate) fn database_name(&self) -> &str {
        &self.database_name
    }
    
    pub(crate) async fn get_document(&self, collection: &str, id: &str) -> Result<Option<mongodb::bson::Document>> {
        let db = self.client.database(&self.database_name);
        let coll= db.collection(collection);
        let filter = mongodb::bson::doc! { "_id": id };
        let document = coll.find_one(filter, None).await?;
        Ok(document)
    }

    pub(crate) async fn insert_document(&self, collection: &str, document: impl Into<mongodb::bson::Document>) -> Result<()> {
        let db = self.client.database(&self.database_name);
        let coll = db.collection(collection);
        coll.insert_one(document.into(), None).await?;
        Ok(())
    }

}