use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use oauth2::{HttpRequest, HttpResponse};
use serde_json::Value;
use tempfile::TempDir;

use crate::oauth2::error::OAuth2Error;

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

    async fn http_request(&self, _request: HttpRequest) -> Result<HttpResponse, OAuth2Error> {
        Ok(self.mock_response.clone())
    }
    async fn send_event(&self, _obj: &str, _event: &str, _result: &Value) -> std::io::Result<()> {
        Ok(())
    }
}

impl Mock {
    pub fn new() -> Self {
        Self {
            token_directory: Arc::new(TempDir::with_prefix_in("tests", ".").unwrap()),

            mock_response: HttpResponse::new(Vec::new()),
        }
    }

    pub fn set_mock_response(mut self, response: HttpResponse) -> Self {
        self.mock_response = response;
        self
    }
}
