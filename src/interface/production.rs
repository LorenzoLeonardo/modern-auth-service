use std::path::PathBuf;

use async_trait::async_trait;
use directories::UserDirs;
use ipc_broker::client::IPCClient;
use oauth2::{HttpRequest, HttpResponse};
use serde_json::Value;

use crate::interface::Interface;
use crate::{
    http_client::HttpClient,
    oauth2::error::{ErrorCodes, OAuth2Error},
};

#[derive(Clone)]
pub struct Production {
    token_directory: PathBuf,
    http_client: HttpClient,
    connector: IPCClient,
}

#[async_trait]
impl Interface for Production {
    fn token_directory(&self) -> PathBuf {
        self.token_directory.clone()
    }

    async fn http_request(&self, request: HttpRequest) -> Result<HttpResponse, OAuth2Error> {
        match &self.http_client {
            HttpClient::Curl(curl) => curl.send(request).await,
            HttpClient::Reqwest(reqwest) => reqwest.send(request).await,
        }
    }

    async fn send_event(&self, object: &str, event: &str, result: &Value) -> std::io::Result<()> {
        self.connector.publish(object, event, result).await
    }
}

impl Production {
    pub fn new(connector: IPCClient, http_client: HttpClient) -> Result<Self, OAuth2Error> {
        let token_directory = UserDirs::new().ok_or(OAuth2Error::new(
            ErrorCodes::DirectoryError,
            "No valid directory".to_string(),
        ))?;
        let mut token_directory = token_directory.home_dir().to_owned();

        token_directory = token_directory.join("token");

        Ok(Self {
            token_directory,
            http_client,
            connector,
        })
    }
}
