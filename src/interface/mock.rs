use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use curl_http_client::error::Error;
use http::{HeaderMap, StatusCode};
use ipc_client::client::message::JsonValue;
use oauth2::{HttpRequest, HttpResponse};
use tempdir::TempDir;

use super::Interface;

#[derive(Clone)]
pub struct Mock {
    token_directory: Arc<TempDir>,
}

#[async_trait]
impl Interface for Mock {
    fn token_directory(&self) -> PathBuf {
        self.token_directory.path().join("token")
    }

    async fn http_request(&self, _request: HttpRequest) -> Result<HttpResponse, Error> {
        Ok(HttpResponse {
            status_code: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Vec::new(),
        })
    }
    async fn send_event(
        &self,
        _event: &str,
        _result: JsonValue,
    ) -> Result<(), ipc_client::client::error::Error> {
        Ok(())
    }
}

impl Mock {
    pub fn new() -> Self {
        Self {
            token_directory: Arc::new(TempDir::new_in(".", "tests").unwrap()),
        }
    }
}
