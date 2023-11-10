// Standard libraries
use std::{
    collections::HashMap,
    fmt::Display,
    future::Future,
    path::{Path, PathBuf},
};

// 3rd party crates
use async_trait::async_trait;
use directories::UserDirs;
use oauth2::{
    basic::{BasicClient, BasicTokenType},
    devicecode::StandardDeviceAuthorizationResponse,
    AuthUrl, ClientId, ClientSecret, DeviceAuthorizationUrl, EmptyExtraTokenFields, HttpRequest,
    HttpResponse, Scope, StandardTokenResponse, TokenUrl,
};

use ipc_client::client::message::JsonValue;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::UnboundedSender, oneshot};

// My crates
use crate::{
    interface::Interface,
    oauth2::{provider::Provider, token_keeper::TokenKeeper},
};
use crate::{
    oauth2::error::{ErrorCodes, OAuth2Error, OAuth2Result},
    task_manager::TaskMessage,
};

#[async_trait]
pub trait DeviceCodeFlowTrait {
    async fn request_device_code<
        F: Future<Output = Result<HttpResponse, RE>> + Send,
        RE: std::error::Error + 'static + Send,
        T: Fn(HttpRequest) -> F + Send + Sync,
    >(
        &self,
        scopes: Vec<Scope>,
        async_http_callback: T,
    ) -> OAuth2Result<StandardDeviceAuthorizationResponse>;
    async fn poll_access_token<
        F: Future<Output = Result<HttpResponse, RE>> + Send,
        RE: std::error::Error + 'static + Send,
        T: Fn(HttpRequest) -> F + Send + Sync,
    >(
        &self,
        device_auth_response: StandardDeviceAuthorizationResponse,
        async_http_callback: T,
    ) -> OAuth2Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>>;
    async fn get_access_token<
        F: Future<Output = Result<HttpResponse, RE>> + Send,
        RE: std::error::Error + 'static + Send,
        T: Fn(HttpRequest) -> F + Send + Sync,
    >(
        &self,
        file_directory: &Path,
        file_name: &Path,
        async_http_callback: T,
    ) -> OAuth2Result<TokenKeeper>;
}

pub struct DeviceCodeFlow {
    client_id: ClientId,
    client_secret: Option<ClientSecret>,
    device_auth_endpoint: DeviceAuthorizationUrl,
    token_endpoint: TokenUrl,
}

#[async_trait]
impl DeviceCodeFlowTrait for DeviceCodeFlow {
    async fn request_device_code<
        F: Future<Output = Result<HttpResponse, RE>> + Send,
        RE: std::error::Error + 'static + Send,
        T: Fn(HttpRequest) -> F + Send + Sync,
    >(
        &self,
        scopes: Vec<Scope>,
        async_http_callback: T,
    ) -> OAuth2Result<StandardDeviceAuthorizationResponse> {
        log::info!(
            "There is no Access token, please login via browser with this link and input the code."
        );
        let client = self
            .create_client()?
            .set_device_authorization_url(self.device_auth_endpoint.to_owned());

        let device_auth_response = client
            .exchange_device_code()?
            .add_scopes(scopes)
            .request_async(async_http_callback)
            .await?;

        Ok(device_auth_response)
    }
    async fn poll_access_token<
        F: Future<Output = Result<HttpResponse, RE>> + Send,
        RE: std::error::Error + 'static + Send,
        T: Fn(HttpRequest) -> F + Send + Sync,
    >(
        &self,
        device_auth_response: StandardDeviceAuthorizationResponse,
        async_http_callback: T,
    ) -> OAuth2Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>> {
        let client = self.create_client()?;
        let token_result = client
            .exchange_device_access_token(&device_auth_response)
            .request_async(async_http_callback, tokio::time::sleep, None)
            .await?;
        log::info!("Access token successfuly retrieved from the endpoint.");
        Ok(token_result)
    }

    async fn get_access_token<
        F: Future<Output = Result<HttpResponse, RE>> + Send,
        RE: std::error::Error + 'static + Send,
        T: Fn(HttpRequest) -> F + Send + Sync,
    >(
        &self,
        file_directory: &Path,
        file_name: &Path,
        async_http_callback: T,
    ) -> OAuth2Result<TokenKeeper> {
        let mut token_keeper = TokenKeeper::new(file_directory.to_path_buf());
        token_keeper.read(file_name)?;

        if token_keeper.has_access_token_expired() {
            match token_keeper.refresh_token {
                Some(ref_token) => {
                    log::info!(
                        "Access token has expired, contacting endpoint to get a new access token."
                    );
                    let response = self
                        .create_client()?
                        .exchange_refresh_token(&ref_token)
                        .request_async(async_http_callback)
                        .await;

                    match response {
                        Ok(res) => {
                            token_keeper = TokenKeeper::from(res);
                            token_keeper.set_directory(file_directory.to_path_buf());
                            token_keeper.save(file_name)?;
                            Ok(token_keeper)
                        }
                        Err(e) => {
                            let error = OAuth2Error::from(e);
                            if error.error_code == ErrorCodes::InvalidGrant {
                                let file = TokenKeeper::new(file_directory.to_path_buf());
                                if let Err(e) = file.delete(file_name) {
                                    log::error!("{:?}", e);
                                }
                            }
                            Err(error)
                        }
                    }
                }
                None => {
                    log::info!("Access token has expired but there is no refresh token, please login again.");
                    token_keeper.delete(file_name)?;
                    Err(OAuth2Error::new(
                        ErrorCodes::NoToken,
                        "There is no refresh token.".into(),
                    ))
                }
            }
        } else {
            Ok(token_keeper)
        }
    }
}

impl DeviceCodeFlow {
    pub fn new(
        client_id: ClientId,
        client_secret: Option<ClientSecret>,
        device_auth_endpoint: DeviceAuthorizationUrl,
        token_endpoint: TokenUrl,
    ) -> Self {
        Self {
            client_id,
            client_secret,
            device_auth_endpoint,
            token_endpoint,
        }
    }

    fn create_client(&self) -> OAuth2Result<BasicClient> {
        Ok(BasicClient::new(
            self.client_id.to_owned(),
            self.client_secret.to_owned(),
            AuthUrl::new(self.token_endpoint.to_owned().to_string())?,
            Some(self.token_endpoint.to_owned()),
        )
        .set_auth_type(oauth2::AuthType::RequestBody))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceCodeFlowParam {
    process: String,
    provider: String,
    scopes: Vec<String>,
}

impl DeviceCodeFlowParam {
    pub fn new(process: String, provider: String, scopes: Vec<String>) -> Self {
        Self {
            process,
            provider,
            scopes,
        }
    }
}

impl Display for DeviceCodeFlowParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.process, self.provider)
    }
}

fn make_token_dir() -> Result<PathBuf, OAuth2Error> {
    let directory = UserDirs::new().ok_or(OAuth2Error::new(
        ErrorCodes::DirectoryError,
        "No valid directory".to_string(),
    ))?;
    let mut directory = directory.home_dir().to_owned();

    directory = directory.join("token");

    Ok(directory)
}

fn make_filename(param: &Provider) -> PathBuf {
    PathBuf::from(format!("{}{}DeviceCodeFlow", param.process, param.provider))
}

pub async fn login<I>(
    provider: Provider,
    interface: I,
    tx: UnboundedSender<TaskMessage>,
) -> Result<StandardDeviceAuthorizationResponse, OAuth2Error>
where
    I: Interface + Send + Sync + 'static + Clone,
{
    log::trace!("login({:?})", provider);

    let token_dir = interface.token_directory();
    let token_file = make_filename(&provider);

    let device_code_flow = DeviceCodeFlow::new(
        provider.client_id,
        provider.client_secret,
        provider.device_auth_endpoint,
        provider.token_endpoint,
    );

    let (oneshot_tx, oneshot_rx) = oneshot::channel();
    tx.send(TaskMessage::Check(token_file.clone(), oneshot_tx))?;
    if let Ok(existing) = oneshot_rx.await {
        if existing {
            tx.send(TaskMessage::Abort(token_file.clone()))?;
            log::info!("task aborted ...");
        }
    }

    let device_auth_response = device_code_flow
        .request_device_code(provider.scopes, |request| async {
            interface.http_request(request).await
        })
        .await?;

    let result = device_auth_response.clone();
    let token_file_clone = token_file.clone();
    let task_channel = tx.clone();
    // Start polling at the background
    let handle = tokio::spawn(async move {
        let result = device_code_flow
            .poll_access_token(device_auth_response, |request| async {
                interface.http_request(request).await
            })
            .await;

        let value = match result {
            Ok(token) => {
                let mut token_keeper = TokenKeeper::from(token);
                token_keeper.set_directory(token_dir);
                if let Err(err) = token_keeper.save(&token_file_clone) {
                    JsonValue::convert_from(&err).unwrap_or_else(|e| {
                        let mut error_hash = HashMap::new();
                        error_hash.insert("error".to_string(), JsonValue::String(e.to_string()));
                        JsonValue::HashMap(error_hash)
                    })
                } else {
                    JsonValue::convert_from(&token_keeper).unwrap_or_else(|e| {
                        let mut error_hash = HashMap::new();
                        error_hash.insert("error".to_string(), JsonValue::String(e.to_string()));
                        JsonValue::HashMap(error_hash)
                    })
                }
            }
            Err(err) => JsonValue::convert_from(&err).unwrap_or_else(|e| {
                let mut error_hash = HashMap::new();
                error_hash.insert("error".to_string(), JsonValue::String(e.to_string()));
                JsonValue::HashMap(error_hash)
            }),
        };

        log::info!("Sending of event . . . .");
        // Sending to event result to the subscribers
        interface
            .send_event("oauth2", value)
            .await
            .unwrap_or_else(|e| {
                log::error!("{:?}", e);
            });
        // Task is done, removing from the list
        task_channel
            .send(TaskMessage::PollingDone(token_file_clone))
            .unwrap_or_else(|e| {
                log::error!("{:?}", e);
            });
        log::info!("Event Sent!!!. . . .");
    });
    // Send this polling task to the background
    tx.send(TaskMessage::Add(token_file, handle))
        .unwrap_or_else(|e| {
            log::error!("{:?}", e);
        });

    Ok(result)
}

pub async fn cancel(
    provider: Provider,
    tx: UnboundedSender<TaskMessage>,
) -> Result<bool, OAuth2Error> {
    log::trace!("cancelLogin({:?})", provider);

    let token_file = make_filename(&provider);
    tx.send(TaskMessage::Abort(token_file))?;
    Ok(true)
}

pub async fn request_token<I>(provider: Provider, interface: I) -> Result<TokenKeeper, OAuth2Error>
where
    I: Interface + Send + Sync + 'static + Clone,
{
    log::trace!("requestToken({:?})", provider);

    let token_dir = interface.token_directory();
    let token_file = make_filename(&provider);

    let device_code_flow = DeviceCodeFlow::new(
        provider.client_id,
        provider.client_secret,
        provider.device_auth_endpoint,
        provider.token_endpoint,
    );

    let token_keeper = device_code_flow
        .get_access_token(&token_dir, &token_file, |request| async {
            interface.http_request(request).await
        })
        .await?;

    Ok(token_keeper)
}

pub async fn logout<I>(provider: Provider, interface: I) -> Result<bool, OAuth2Error>
where
    I: Interface + Send + Sync + 'static + Clone,
{
    log::trace!("logout({:?})", provider);

    let token_dir = interface.token_directory();
    let token_file = make_filename(&provider);

    let token_keeper = TokenKeeper::new(token_dir);

    token_keeper.delete(token_file.as_path())?;
    Ok(true)
}
