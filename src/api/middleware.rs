use super::API;
use futures::future::BoxFuture;
use tide::{
    http::headers::HeaderValue,
    security::{CorsMiddleware, Origin},
    Middleware, Next, Request, Result,
};
impl API {
    pub fn middleware(&mut self) {
        self.app.middleware(cors_middleware());
        self.app.middleware(cbor_middleware());
    }
}
pub fn cors_middleware() -> CorsMiddleware {
    CorsMiddleware::new()
        .allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"))
        .allow_credentials(false)
}

pub fn cbor_middleware() -> CborMiddleware {
    CborMiddleware::new()
}
impl CborMiddleware {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct CborMiddleware;
impl<State: Send + Sync + 'static> Middleware<State> for CborMiddleware {
    fn handle<'a>(&'a self, req: Request<State>, next: Next<'a, State>) -> BoxFuture<'a, Result> {
        Box::pin(async move {
            let mut response = next.run(req).await?;
            response.insert_header("Content-Type", "application/cbor");
            Ok(response)
        })
    }
}
