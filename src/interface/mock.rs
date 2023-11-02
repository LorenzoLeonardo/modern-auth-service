use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use curl_http_client::error::Error;
use http::{HeaderMap, StatusCode};
use ipc_client::client::message::JsonValue;
use oauth2::{HttpRequest, HttpResponse};
use tempdir::TempDir;

use crate::oauth2::provider::Provider;

use super::Interface;

#[derive(Clone)]
pub struct Mock {
    token_directory: Arc<TempDir>,
    provider_directory: Arc<TempDir>,
}

#[async_trait]
impl Interface for Mock {
    fn token_directory(&self) -> PathBuf {
        self.token_directory.path().join("token")
    }

    fn provider_directory(&self) -> PathBuf {
        self.provider_directory.path().join("endpoints")
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
    pub fn new(mock_provider: Provider, provider_file: PathBuf) -> Self {
        let provider_directory = Arc::new(TempDir::new_in(".", "tests").unwrap());
        let dir = provider_directory.path().join("endpoints");
        fs::create_dir_all(dir.as_path()).unwrap();

        let file_path = dir.join(provider_file);
        let mut file = File::create(file_path).unwrap();

        let contents = serde_json::to_string(&mock_provider).unwrap();
        file.write_all(contents.as_bytes()).unwrap();

        Self {
            token_directory: Arc::new(TempDir::new_in(".", "tests").unwrap()),
            provider_directory,
        }
    }
}
