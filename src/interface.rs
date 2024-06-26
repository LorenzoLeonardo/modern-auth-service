pub mod curl;
#[cfg(test)]
pub mod mock;
pub mod production;

use std::path::PathBuf;

use async_trait::async_trait;

use curl_http_client::collector::Collector;
use json_elem::jsonelem::JsonElem;
use oauth2::{HttpRequest, HttpResponse};
use remote_call::RemoteError;

#[async_trait]
pub trait Interface {
    fn token_directory(&self) -> PathBuf;
    async fn http_request(
        &self,
        request: HttpRequest,
    ) -> Result<HttpResponse, curl_http_client::error::Error<Collector>>;
    async fn send_event(&self, event: &str, result: JsonElem) -> Result<(), RemoteError>;
}
