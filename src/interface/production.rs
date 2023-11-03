use std::path::PathBuf;

use async_trait::async_trait;
use curl_http_client::error::Error;
use directories::UserDirs;
use ipc_client::client::{connector::Connector, message::JsonValue};
use oauth2::{HttpRequest, HttpResponse};

use crate::oauth2::error::{ErrorCodes, OAuth2Error};

use super::{curl::Curl, Interface};

#[derive(Clone)]
pub struct Production {
    token_directory: PathBuf,
    curl: Curl,
    connector: Connector,
}

#[async_trait]
impl Interface for Production {
    fn token_directory(&self) -> PathBuf {
        self.token_directory.clone()
    }

    async fn http_request(&self, request: HttpRequest) -> Result<HttpResponse, Error> {
        self.curl.send(request).await
    }

    async fn send_event(
        &self,
        event: &str,
        result: JsonValue,
    ) -> Result<(), ipc_client::client::error::Error> {
        self.connector.send_event(event, result).await
    }
}

impl Production {
    pub fn new(connector: Connector) -> Result<Self, OAuth2Error> {
        let token_directory = UserDirs::new().ok_or(OAuth2Error::new(
            ErrorCodes::DirectoryError,
            "No valid directory".to_string(),
        ))?;
        let mut token_directory = token_directory.home_dir().to_owned();

        token_directory = token_directory.join("token");

        Ok(Self {
            token_directory,
            curl: Curl::new(),
            connector,
        })
    }
}
