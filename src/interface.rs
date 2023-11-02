pub mod curl;
#[cfg(test)]
pub mod mock;
pub mod production;

use std::path::PathBuf;

use async_trait::async_trait;
use curl_http_client::error::Error;
use oauth2::{HttpRequest, HttpResponse};

#[async_trait]
pub trait Interface {
    fn token_directory(&self) -> PathBuf;
    fn provider_directory(&self) -> PathBuf;
    async fn http_request(&self, request: HttpRequest) -> Result<HttpResponse, Error>;
}
