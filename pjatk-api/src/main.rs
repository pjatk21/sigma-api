use futures::stream::TryStreamExt;
use mongodb::bson::{DateTime, Bson};
use timetable::TimeTableEntry;

use mongodb::{bson::doc, options::ClientOptions, Client, Collection, Cursor};

use poem::{listener::TcpListener, web::Data, EndpointExt, Route, Server};
use poem_openapi::param::Query;
use poem_openapi::{payload::Json, OpenApi, OpenApiService};

use std::error::Error;
use std::ops::Deref;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let coll_db = connect_db().await?;
    let api_service =
        OpenApiService::new(Api, "PJATK Schedule API", "0.1").server("https://sigmaapi.justsomebody.dev:3001/api");
    let docs = api_service.swagger_ui();
    let open_api_specs = api_service.spec_endpoint();
    let app = Route::new()
        .nest("/", docs)
        .nest("/api", api_service)
        .nest("/openapi.json", open_api_specs)
        .data(coll_db.clone());
    Server::new(TcpListener::bind("0.0.0.0:3001"))
        .run(app)
        .await?;
    Ok(())
}
async fn connect_db() -> Result<Collection<TimeTableEntry>, Box<dyn Error>> {
    let url = format!(
        "mongodb://{0}:{1}@mongodb:27017",
        std::env::var("MONGO_INITDB_ROOT_USERNAME")?,
        std::env::var("MONGO_INITDB_ROOT_PASSWORD")?
    );
    let mut client_options = ClientOptions::parse(url).await.expect("Bad mongo url!");
    client_options.app_name = Some("PJATK Schedule".to_string());
    let client_db = Client::with_options(client_options).expect("Client failed!");
    let db = client_db.database("schedule");
    let coll: Collection<TimeTableEntry> = db.collection("timetable_entries");
    Ok(coll)
}

struct Api;
#[OpenApi]
impl Api {
    #[oai(path = "/get_timetable", method = "get")]
    async fn get_timetable(
        &self,
        coll_db: Data<&Collection<TimeTableEntry>>,
        date_from: Query<Option<i64>>,
        date_to: Query<Option<i64>>,
        group: Query<Option<String>>,
    ) -> Json<Vec<TimeTableEntry>> {
        let cursor: Cursor<TimeTableEntry> = coll_db
            .find(
                match (date_from.deref(), date_to.deref(), group.deref()) {
                    (Some(date_from), Some(date_to), Some(group)) => {
                        let datetime_beginning= DateTime::from_millis(*date_from * 1000);
                        let datetime_ending= DateTime::from_millis(*date_to * 1000);
                        doc! {"datetime_beginning":{"$gte":Bson::DateTime(datetime_beginning)},"datetime_ending":{"$lte":Bson::DateTime(datetime_ending)},"groups":group}
                    },
                    (Some(date_from), Some(date_to), None) => {
                        let datetime_beginning= DateTime::from_millis(*date_from * 1000);
                        let datetime_ending= DateTime::from_millis(*date_to * 1000);
                        doc! {"datetime_beginning":{"$gte":Bson::DateTime(datetime_beginning)},"datetime_ending":{"$lte":Bson::DateTime(datetime_ending)}}},
                    (Some(date_from), None, Some(group)) => {
                        let datetime_beginning= DateTime::from_millis(*date_from * 1000);
                        doc! {"datetime_beginning":{"$gte":Bson::DateTime(datetime_beginning)},"groups":group}
                    },
                    (Some(date_from), None, None) => {
                        let datetime_beginning= DateTime::from_millis(*date_from * 1000);
                        doc! {"datetime_beginning":{"$gte":Bson::DateTime(datetime_beginning)}}
                    },
                    (None, Some(date_to), Some(group)) => {
                        let datetime_ending= DateTime::from_millis(*date_to * 1000);
                        doc! {"datetime_ending":{"$lte":Bson::DateTime(datetime_ending)},"groups":group}
                    },
                    (None, Some(date_to), None) => {
                        let datetime_ending= DateTime::from_millis(*date_to * 1000);
                        doc! {"datetime_ending":{"$lte":Bson::DateTime(datetime_ending)}}
                    },
                    (None, None, Some(group)) => doc! {"groups":group},
                    (None, None, None) => doc! {},
                },
                None,
            )
            .await
            .expect("find failed!");
        let entries: Vec<TimeTableEntry> = cursor.try_collect().await.expect("collect failed!");
        Json(entries)
    }
}
