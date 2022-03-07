#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]

use api_utils::SigmaApiData;
use api_utils::SigmaApiError;
use api_utils::SigmaApiResponse;
use futures::stream::TryStreamExt;
use mongodb::bson::{Bson, DateTime};

use poem::middleware::TowerLayerCompatExt;
use poem::EndpointExt;

use serde::Deserialize;
use timetable::TimeTableEntry;

use mongodb::{bson::doc, Collection, Cursor};

use poem::{listener::TcpListener, web::Data, Route, Server};
use poem_openapi::param::Query;
use poem_openapi::{payload::Json, OpenApi, OpenApiService};

use config::Config;
use std::error::Error as StdError;

use std::ops::Deref;
use std::time::Duration;
use tracing::{error, Level};
use tracing_subscriber::FmtSubscriber;

mod config;
#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    let config = Config::new().await?;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    let coll_db = config.get_collection().await?;
    let port = config.get_port();
    let server_url = config.get_complete_server_url();
    let api_service = OpenApiService::new(Api, "PJATK Schedule API", "0.4.2").server(server_url);
    let docs = api_service.redoc();
    let open_api_specs = api_service.spec_endpoint();
    let app = Route::new()
        .nest("/", docs)
        .nest("/api", api_service)
        .nest("/openapi.json", open_api_specs)
        .data(coll_db.clone())
        .with(tower::limit::RateLimitLayer::new(5, Duration::from_secs(1)).compat())
        .with(poem::middleware::Tracing)
        .catch_all_error(SigmaApiError::handle_error);
    Server::new(TcpListener::bind(format!("0.0.0.0:{}", port)))
        .run(app)
        .await?;
    Ok(())
}

#[derive(Deserialize, PartialEq, Eq, Hash, Clone)]

struct ApiParams {
    date_from: Option<i64>,
    date_to: Option<i64>,
    groups: Option<String>,
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
        /// Array of tutors to only search for - seperated by `;`
        tutors: Query<Option<String>>,
    ) -> SigmaApiResponse<Vec<TimeTableEntry>, SigmaApiError> {
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
        } else if let Some(tutors) = tutors.deref() {
            for tutor in tutors
                .split_terminator(';')
                .filter(|tutor| !tutor.is_empty())
            {
                let cursor: Cursor<TimeTableEntry> = coll_db
            .find(
                match (date_from.deref(), date_to.deref()) {
                    (Some(date_from), Some(date_to)) => {
                        let datetime_beginning= DateTime::from_millis(*date_from * 1000);
                        let datetime_ending= DateTime::from_millis(*date_to * 1000);
                        doc! {"datetime_beginning":{"$gte":Bson::DateTime(datetime_beginning)},"datetime_ending":{"$lte":Bson::DateTime(datetime_ending)},"persons":tutor}
                    },
                    (Some(date_from), None) => {
                        let datetime_beginning= DateTime::from_millis(*date_from * 1000);
                        doc! {"datetime_beginning":{"$gte":Bson::DateTime(datetime_beginning)},"persons":tutor}
                    },
                    (None, Some(date_to)) => {
                        let datetime_ending= DateTime::from_millis(*date_to * 1000);
                        doc! {"datetime_ending":{"$lte":Bson::DateTime(datetime_ending)},"persons":tutor}
                    },
                    (None, None) => doc! {"persons":tutor},
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
            error!("{}", "No entries found!");
            SigmaApiResponse::NotFound(Json(
                SigmaApiError::error(404, "No entries found!".to_string(), None)
                    .expect("Error failed!"),
            ))
        } else {
            entries.sort_by_key(|a| a.get_datetime_beginning());
            SigmaApiResponse::Found(Json(SigmaApiData::new(entries)))
        }
    }

    /// Get all avaliable groups
    #[oai(path = "/get_groups", method = "get")]
    async fn get_groups(
        &self,
        coll_db: Data<&Collection<TimeTableEntry>>,
    ) -> SigmaApiResponse<Vec<String>, SigmaApiError> {
        if let Ok(cursor) = coll_db.distinct("groups", None, None).await {
            let groups: Vec<String> = cursor
                .into_iter()
                .map(|entry| entry.to_string().replace('\"', ""))
                .collect();
            if groups.is_empty() {
                error!("{}", "No groups found!");
                SigmaApiResponse::NotFound(Json(
                    SigmaApiError::error(404, "No groups found!".to_string(), None)
                        .expect("Error failed!"),
                ))
            } else {
                SigmaApiResponse::Found(Json(SigmaApiData::new(groups)))
            }
        } else {
            error!("{}", "MongoDB error!");
            SigmaApiResponse::InternalError(Json(
                SigmaApiError::error(500, "MongoDB Error!".to_string(), None)
                    .expect("Error failed!"),
            ))
        }
    }
    /// Get all avaliable tutors
    #[oai(path = "/get_tutors", method = "get")]
    async fn get_tutors(
        &self,
        coll_db: Data<&Collection<TimeTableEntry>>,
    ) -> SigmaApiResponse<Vec<String>, SigmaApiError> {
        if let Ok(cursor) = coll_db.distinct("persons", None, None).await {
            let tutors: Vec<String> = cursor
                .into_iter()
                .flat_map(|entries| {
                    if let Some(found_entries) = entries.as_array() {
                        found_entries
                            .iter()
                            .map(|entry| entry.to_string().replace('\"', ""))
                            .collect()
                    } else {
                        vec![entries.to_string().replace('\"', "")]
                    }
                })
                .collect();
            if tutors.is_empty() {
                error!("{}", "No tutors found!");
                SigmaApiResponse::NotFound(Json(
                    SigmaApiError::error(404, "No tutors found!".to_string(), None)
                        .expect("Error failed!"),
                ))
            } else {
                SigmaApiResponse::Found(Json(SigmaApiData::new(tutors)))
            }
        } else {
            error!("{}", "MongoDB error!");
            SigmaApiResponse::InternalError(Json(
                SigmaApiError::error(500, "MongoDB Error!".to_string(), None)
                    .expect("Error failed!"),
            ))
        }
    }
}
