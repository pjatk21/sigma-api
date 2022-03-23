// #![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
// use poem::{Error, IntoResponse};

// use serde::{Deserialize, Serialize};

// use mongodb::bson::doc;

// use poem_openapi::payload::Json;
// use poem_openapi::{types::*, Object};
// use tracing::error;

// use std::error::Error as StdError;
// use std::fmt::Display;

// #[derive(poem_openapi::ApiResponse)]
// /// Sigma API Response
// pub enum SigmaApiResponse<T: Send + Sync + ToJSON + ParseFromJSON, E: Send + ToJSON + StdError> {
//     /// Found data
//     #[oai(status = 200)]
//     Found(Json<SigmaApiData<T>>),
//     /// Nothing was found
//     #[oai(status = 404)]
//     NotFound(Json<E>),
//     /// User send out bad request
//     #[oai(status = 400)]
//     BadRequest(Json<E>),
//     /// Server encountered internal error
//     #[oai(status = 500)]
//     InternalError(Json<E>),
// }
// #[derive(Deserialize, Serialize, Debug, Object)]
// #[oai(inline)]
// pub struct SigmaApiData<T: Send + Sync + ToJSON + ParseFromJSON> {
//     data: T,
// }

// impl<T: Send + Sync + ToJSON + ParseFromJSON> SigmaApiData<T> {
//     pub fn new(data: T) -> Self {
//         Self { data }
//     }
// }

// #[derive(Object, Serialize, Deserialize, Debug, Clone)]
// #[oai(inline)]
// pub struct SigmaApiError {
//     code: u16,
//     name: String,
//     cause: Option<String>,
// }

// impl SigmaApiError {
//     pub fn error(code: u16, name: String, err: Option<String>) -> Result<Self, Box<dyn StdError>> {
//         Ok(Self {
//             code,
//             name,
//             cause: err,
//         })
//     }
//     #[tracing::instrument]
//     pub async fn handle_error(err: Error) -> impl IntoResponse {
//         error!("{:?}", err);
//         let status = err.as_response().status();
//         let cause_err = SigmaApiError::error(
//             status.as_u16(),
//             status.as_str().to_owned(),
//             Some(err.to_string()),
//         )
//         .expect("API error from poem::Error failed!");
//         Json(cause_err).with_status(status)
//     }
// }

// impl StdError for SigmaApiError {
//     fn source(&self) -> Option<&(dyn StdError + 'static)> {
//         None
//     }

//     fn cause(&self) -> Option<&dyn StdError> {
//         self.source()
//     }
// }
// impl Display for SigmaApiError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "{0}: {1} - {2}",
//             self.code,
//             self.name,
//             self.cause.as_ref().unwrap_or(&"No known cause".to_string())
//         )
//     }
// }
