#![deny(clippy::perf, clippy::complexity, clippy::style)]

use futures::stream::TryStreamExt;
use mongodb::bson::{Bson, DateTime};

use poem_openapi::payload::PlainText;
use serde::{Deserialize, Serialize};
use timetable::TimeTableEntry;

use mongodb::{bson::doc, options::ClientOptions, Client, Collection, Cursor};

use poem::{listener::TcpListener, web::Data, EndpointExt, Route, Server};
use poem_openapi::param::Query;
use poem_openapi::{payload::Json, OpenApi, OpenApiService};
use poem_openapi::{types::*, Object};
use std::error::Error;
use std::fmt::Display;
use std::ops::Deref;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let coll_db = connect_db().await?;
    let port = std::env::var("PJATK_API_PORT")?;
    let server_url = format!(
        "{0}:{1}/api",
        std::env::var("PJATK_API_URL_WITH_PROTOCOL")?,
        port
    );
    let api_service = OpenApiService::new(Api, "PJATK Schedule API", "0.2").server(server_url);
    let docs = api_service.redoc();
    let open_api_specs = api_service.spec_endpoint();
    let app = Route::new()
        .nest("/", docs)
        .nest("/api", api_service)
        .nest("/openapi.json", open_api_specs)
        .data(coll_db.clone());
    Server::new(TcpListener::bind(format!("0.0.0.0:{}", port)))
        .run(app)
        .await?;
    Ok(())
}
async fn connect_db() -> Result<Collection<TimeTableEntry>, Box<dyn Error>> {
    let url = format!(
        "mongodb://{0}:{1}@{2}:{3}",
        std::env::var("MONGO_INITDB_ROOT_USERNAME")?,
        std::env::var("MONGO_INITDB_ROOT_PASSWORD")?,
        std::env::var("MONGO_HOST")?,
        std::env::var("MONGO_PORT")?,
    );
    let mut client_options = ClientOptions::parse(url).await.expect("Bad mongo url!");
    client_options.app_name = Some("PJATK Schedule".to_string());
    let client_db = Client::with_options(client_options).expect("Client failed!");
    let db = client_db.database(&std::env::var("MONGO_INITDB_DATABASE")?);
    let coll: Collection<TimeTableEntry> =
        db.collection(&std::env::var("MONGO_INITDB_COLLECTION")?);
    Ok(coll)
}

struct Api;
#[OpenApi]
impl Api {
    /// Get an timetable
    #[oai(path = "/get_timetable", method = "get")]
    async fn get_timetable(
        &self,
        coll_db: Data<&Collection<TimeTableEntry>>,
        /// Unix timestamp - beginning of search
        date_from: Query<Option<i64>>,
        /// Unix timestamp - end of search
        date_to: Query<Option<i64>>,
        /// Array of groups to only search for - seperated by `;`
        groups: Query<Option<String>>,
    ) -> SigmaApiResponse<TimeTableEntry, SigmaApiError> {
        let mut entries: Vec<TimeTableEntry> = vec![];
        if let Some(groups) = groups.deref() {
            for group in groups
                .split_terminator(';')
                .filter(|group| !group.is_empty())
            {
                let cursor: Cursor<TimeTableEntry> = coll_db
                .find(
                    match (date_from.deref(), date_to.deref()) {
                        (Some(date_from), Some(date_to)) => {
                            let datetime_beginning= DateTime::from_millis(*date_from * 1000);
                            let datetime_ending= DateTime::from_millis(*date_to * 1000);
                            doc! {"datetime_beginning":{"$gte":Bson::DateTime(datetime_beginning)},"datetime_ending":{"$lte":Bson::DateTime(datetime_ending)},"groups":group}
                        },
                        (Some(date_from), None) => {
                            let datetime_beginning= DateTime::from_millis(*date_from * 1000);
                            doc! {"datetime_beginning":{"$gte":Bson::DateTime(datetime_beginning)},"groups":group}
                        },
                        (None, Some(date_to)) => {
                            let datetime_ending= DateTime::from_millis(*date_to * 1000);
                            doc! {"datetime_ending":{"$lte":Bson::DateTime(datetime_ending)},"groups":group}
                        },
                        (None, None) => doc! {"groups":group},
                    },
                    None,
                )
                .await
                .expect("find failed!");
                let mut group_entries: Vec<TimeTableEntry> =
                    cursor.try_collect().await.expect("collect failed!");
                entries.append(&mut group_entries);
            }
        } else {
            let cursor: Cursor<TimeTableEntry> = coll_db
            .find(
                match (date_from.deref(), date_to.deref()) {
                    (Some(date_from), Some(date_to)) => {
                        let datetime_beginning= DateTime::from_millis(*date_from * 1000);
                        let datetime_ending= DateTime::from_millis(*date_to * 1000);
                        doc! {"datetime_beginning":{"$gte":Bson::DateTime(datetime_beginning)},"datetime_ending":{"$lte":Bson::DateTime(datetime_ending)}}},
                    (Some(date_from), None) => {
                        let datetime_beginning= DateTime::from_millis(*date_from * 1000);
                        doc! {"datetime_beginning":{"$gte":Bson::DateTime(datetime_beginning)}}
                    },
                    (None, Some(date_to)) => {
                        let datetime_ending= DateTime::from_millis(*date_to * 1000);
                        doc! {"datetime_ending":{"$lte":Bson::DateTime(datetime_ending)}}
                    },
                    (None, None) => doc! {},
                },
                None,
            )
            .await
            .expect("find failed!");
            entries = cursor.try_collect().await.expect("collect failed!");
        }
        if entries.is_empty() {
            SigmaApiResponse::NotFound(PlainText("Not Found".to_string()))
        } else {
            entries.sort_by_key(|a| a.get_datetime_beginning());
            SigmaApiResponse::Found(Json(entries))
        }
    }

    /// Get all avaliable groups
    #[oai(path = "/get_groups", method = "get")]
    async fn get_groups(
        &self,
        coll_db: Data<&Collection<TimeTableEntry>>,
    ) -> SigmaApiResponse<String, SigmaApiError> {
        if let Ok(cursor) = coll_db.distinct("groups", None, None).await {
            let groups: Vec<String> = cursor
                .into_iter()
                .map(|entry| entry.to_string().replace('\"', ""))
                .collect();
            SigmaApiResponse::Found(Json(groups))
        } else {
            SigmaApiResponse::NotFound(PlainText("Not Found".to_string()))
        }
    }
}

#[derive(poem_openapi::ApiResponse)]
enum SigmaApiResponse<T: Send + ToJSON, E: Send + ToJSON + Error> {
    #[oai(status = 200)]
    Found(Json<Vec<T>>),
    #[oai(status = 404)]
    NotFound(PlainText<String>),
    #[oai(status = 500)]
    InternalError(Json<E>),
}

#[derive(Object, Serialize, Deserialize, Debug, Clone)]
struct SigmaApiError {}

impl Error for SigmaApiError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}
impl Display for SigmaApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Internal Server Error")
    }
}
