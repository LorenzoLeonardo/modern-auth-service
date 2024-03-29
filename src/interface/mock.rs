use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use curl_http_client::{collector::Collector, error::Error};
use http::{HeaderMap, StatusCode};
use json_elem::jsonelem::JsonElem;
use oauth2::{HttpRequest, HttpResponse};
use remote_call::RemoteError;
use tempfile::TempDir;

use super::Interface;

#[derive(Clone)]
pub struct Mock {
    token_directory: Arc<TempDir>,
    mock_response: HttpResponse,
}

#[async_trait]
impl Interface for Mock {
    fn token_directory(&self) -> PathBuf {
        self.token_directory.path().join("token")
    }

    async fn http_request(&self, _request: HttpRequest) -> Result<HttpResponse, Error<Collector>> {
        Ok(self.mock_response.clone())
    }
    async fn send_event(&self, _event: &str, _result: JsonElem) -> Result<(), RemoteError> {
        Ok(())
    }
}

impl Mock {
    pub fn new() -> Self {
        Self {
            token_directory: Arc::new(TempDir::with_prefix_in("tests", ".").unwrap()),
            mock_response: HttpResponse {
                status_code: StatusCode::OK,
                headers: HeaderMap::new(),
                body: Vec::new(),
            },
        }
    }

    pub fn set_mock_response(mut self, response: HttpResponse) -> Self {
        self.mock_response = response;
        self
    }
}
