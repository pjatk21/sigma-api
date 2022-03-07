#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use poem::{
    http::StatusCode,
    web::headers::{self, authorization::Bearer, HeaderMapExt},
    Endpoint, Error, Middleware, Request, Result,
};
use tracing::trace;

use crate::config::ENVIROMENT;

pub(crate) struct BearerAuth {
    token: String,
}

impl BearerAuth {
    pub(crate) fn new() -> Self {
        Self {
            token: ENVIROMENT.AUTH_KEY.to_owned(),
        }
    }
}

impl<E: Endpoint> Middleware<E> for BearerAuth {
    type Output = BearerAuthEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        BearerAuthEndpoint {
            ep,
            token: std::env::var(self.token.clone()).expect("Missing Auth key!"),
        }
    }
}
#[derive(Debug)]
pub(crate) struct BearerAuthEndpoint<E> {
    ep: E,
    token: String,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for BearerAuthEndpoint<E> {
    type Output = E::Output;
    async fn call(&self, req: Request) -> Result<Self::Output> {
        let path = req.uri().path();
        trace!("{0}", path);
        if let Some(auth) = req.headers().typed_get::<headers::Authorization<Bearer>>() {
            trace!("{0} == {1} ?", auth.0.token(), self.token);
            if auth.0.token() == self.token {
                return self.ep.call(req).await;
            }
        } else if req.uri().path() == "/openapi.json" || req.uri().path() == "/" {
            return self.ep.call(req).await;
        }
        Err(Error::from_status(StatusCode::UNAUTHORIZED))
    }
}
