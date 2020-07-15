use super::API;
use tide::{
    http::headers::HeaderValue,
    security::{CorsMiddleware, Origin},
};
impl API {
    pub fn middleware(&mut self) {
        self.app.with(cors_middleware());
    }
}
pub fn cors_middleware() -> CorsMiddleware {
    CorsMiddleware::new()
        .allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"))
        .allow_credentials(false)
}
