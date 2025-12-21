#[cfg(test)]
pub mod mock;
pub mod production;

use std::path::PathBuf;

use async_trait::async_trait;

use oauth2::{HttpRequest, HttpResponse};
use serde_json::Value;

use crate::oauth2::error::OAuth2Error;

#[async_trait]
pub trait Interface {
    fn token_directory(&self) -> PathBuf;
    async fn http_request(&self, request: HttpRequest) -> Result<HttpResponse, OAuth2Error>;
    async fn send_event(&self, obj: &str, event: &str, result: &Value) -> std::io::Result<()>;
}
