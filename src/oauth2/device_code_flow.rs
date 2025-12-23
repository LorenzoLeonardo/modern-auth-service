// Standard libraries
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

// 3rd party crates
use async_trait::async_trait;
use directories::UserDirs;
use json_result::r#struct::JsonResult;
use oauth2::{
    basic::{
        BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
        BasicTokenType,
    },
    Client, ClientId, ClientSecret, DeviceAuthorizationUrl, EndpointNotSet, ExtraTokenFields,
    Scope, StandardDeviceAuthorizationResponse, StandardRevocableToken, StandardTokenResponse,
    TokenUrl,
};

use openidconnect::core::CoreIdToken;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::UnboundedSender, oneshot};

// My crates
use crate::{
    http_client::OAuth2Client,
    interface::Interface,
    oauth2::{provider::InputParameters, token_keeper::TokenKeeper},
};
use crate::{
    oauth2::error::{ErrorCodes, OAuth2Error, OAuth2Result},
    task_manager::TaskMessage,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct CustomExtraFields {
    pub id_token: Option<CoreIdToken>,
}

pub type CustomTokenResponse = StandardTokenResponse<CustomExtraFields, BasicTokenType>;

impl ExtraTokenFields for CustomExtraFields {}

pub type CustomClient<
    HasAuthUrl = EndpointNotSet,
    HasDeviceAuthUrl = EndpointNotSet,
    HasIntrospectionUrl = EndpointNotSet,
    HasRevocationUrl = EndpointNotSet,
    HasTokenUrl = EndpointNotSet,
> = Client<
    BasicErrorResponse,
    CustomTokenResponse,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
>;

#[async_trait]
pub trait DeviceCodeFlowTrait {
    async fn request_device_code<I: Interface + Send + Sync + Clone + 'static>(
        &self,
        scopes: Vec<Scope>,
        interface: I,
    ) -> OAuth2Result<StandardDeviceAuthorizationResponse>;
    async fn poll_access_token<I: Interface + Send + Sync + Clone + 'static>(
        &self,
        device_auth_response: StandardDeviceAuthorizationResponse,
        interface: I,
        tx: UnboundedSender<TaskMessage>,
    ) -> OAuth2Result<CustomTokenResponse>;
    async fn get_access_token<I: Interface + Send + Sync + Clone + 'static>(
        &self,
        file_directory: &Path,
        file_name: &Path,
        interface: I,
    ) -> OAuth2Result<TokenKeeper>;
}

pub struct DeviceCodeFlow {
    client_id: ClientId,
    client_secret: Option<ClientSecret>,
    device_auth_endpoint: DeviceAuthorizationUrl,
    token_endpoint: TokenUrl,
    tx: UnboundedSender<TaskMessage>,
}

#[async_trait]
impl DeviceCodeFlowTrait for DeviceCodeFlow {
    async fn request_device_code<I: Interface + Send + Sync + Clone + 'static>(
        &self,
        scopes: Vec<Scope>,
        interface: I,
    ) -> OAuth2Result<StandardDeviceAuthorizationResponse> {
        log::info!(
            "There is no Access token, please login via browser with this link and input the code."
        );
        let mut client = CustomClient::new(self.client_id.to_owned());
        if let Some(client_secret) = self.client_secret.to_owned() {
            client = client.set_client_secret(client_secret);
        }
        let http_client = OAuth2Client::new(interface, None);
        let device_auth_response = client
            .set_auth_type(oauth2::AuthType::RequestBody)
            .set_token_uri(self.token_endpoint.to_owned())
            .set_device_authorization_url(self.device_auth_endpoint.to_owned())
            .exchange_device_code()
            .add_scopes(scopes)
            .request_async(&http_client)
            .await?;

        Ok(device_auth_response)
    }
    async fn poll_access_token<I: Interface + Send + Sync + Clone + 'static>(
        &self,
        device_auth_response: StandardDeviceAuthorizationResponse,
        interface: I,
        tx: UnboundedSender<TaskMessage>,
    ) -> OAuth2Result<CustomTokenResponse> {
        let mut client = CustomClient::new(self.client_id.to_owned());
        if let Some(client_secret) = self.client_secret.to_owned() {
            client = client.set_client_secret(client_secret);
        }
        let async_http_callback = OAuth2Client::new(interface.clone(), Some(tx));
        let token_result = client
            .set_auth_type(oauth2::AuthType::RequestBody)
            .set_token_uri(self.token_endpoint.to_owned())
            .exchange_device_access_token(&device_auth_response)
            .request_async(&async_http_callback, tokio::time::sleep, None)
            .await?;

        log::info!("Access token successfuly retrieved from the endpoint.");
        Ok(token_result)
    }

    async fn get_access_token<I: Interface + Send + Sync + Clone + 'static>(
        &self,
        file_directory: &Path,
        file_name: &Path,
        interface: I,
    ) -> OAuth2Result<TokenKeeper> {
        let mut token_keeper = TokenKeeper::new(file_directory.to_path_buf());
        token_keeper.read(file_name)?;

        if token_keeper.has_access_token_expired() {
            match token_keeper.refresh_token {
                Some(ref_token) => {
                    log::info!(
                        "Access token has expired, contacting endpoint to get a new access token."
                    );
                    let mut client = CustomClient::new(self.client_id.to_owned());
                    if let Some(client_secret) = self.client_secret.to_owned() {
                        client = client.set_client_secret(client_secret);
                    }
                    let async_http_callback = OAuth2Client::new(interface.clone(), None);
                    let response = client
                        .set_auth_type(oauth2::AuthType::RequestBody)
                        .set_token_uri(self.token_endpoint.to_owned())
                        .exchange_refresh_token(&ref_token)
                        .request_async(&async_http_callback)
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
        tx: UnboundedSender<TaskMessage>,
    ) -> Self {
        Self {
            client_id,
            client_secret,
            device_auth_endpoint,
            token_endpoint,
            tx,
        }
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

fn make_filename(param: &InputParameters) -> Result<PathBuf, OAuth2Error> {
    Ok(PathBuf::from(format!(
        "{}{}DeviceCodeFlow",
        param.process.clone().ok_or(OAuth2Error::new(
            ErrorCodes::ParseError,
            "No Process Name supplied.".into(),
        ))?,
        param.provider.clone().ok_or(OAuth2Error::new(
            ErrorCodes::ParseError,
            "No Provider Name supplied.".into(),
        ))?
    )))
}

pub async fn login<I>(
    provider: InputParameters,
    interface: I,
    tx: UnboundedSender<TaskMessage>,
) -> Result<StandardDeviceAuthorizationResponse, OAuth2Error>
where
    I: Interface + Send + Sync + 'static + Clone,
{
    log::trace!("login({:?})", provider);

    let token_dir = interface.token_directory();
    let token_file = make_filename(&provider)?;

    let device_code_flow = DeviceCodeFlow::new(
        provider.client_id.ok_or(OAuth2Error::new(
            ErrorCodes::ParseError,
            "No Client ID supplied.".into(),
        ))?,
        provider.client_secret,
        provider.device_auth_endpoint.ok_or(OAuth2Error::new(
            ErrorCodes::ParseError,
            "No Device Auth URL supplied.".into(),
        ))?,
        provider.token_endpoint.ok_or(OAuth2Error::new(
            ErrorCodes::ParseError,
            "No Token URL supplied.".into(),
        ))?,
        tx.clone(),
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
        .request_device_code(
            provider.scopes.ok_or(OAuth2Error::new(
                ErrorCodes::ParseError,
                "No Scopes supplied.".into(),
            ))?,
            interface.clone(),
        )
        .await?;

    let result = device_auth_response.clone();
    let token_file_clone = token_file.clone();
    let task_channel = tx.clone();
    // Start polling at the background
    let handle = tokio::spawn(async move {
        let result = device_code_flow
            .poll_access_token(
                device_auth_response,
                interface.clone(),
                task_channel.clone(),
            )
            .await;

        let value = match result {
            Ok(token) => {
                let mut token_keeper = TokenKeeper::from(token);
                token_keeper.set_directory(token_dir);
                if let Err(err) = token_keeper.save(&token_file_clone) {
                    JsonResult::<(), OAuth2Error>(Err(err)).into()
                } else {
                    JsonResult::<TokenKeeper, OAuth2Error>(Ok(token_keeper)).into()
                }
            }
            Err(err) => {
                log::error!("{err}");
                JsonResult::<(), OAuth2Error>(Err(err)).into()
            }
        };

        log::info!("Sending of event . . . .");
        // Sending to event result to the subscribers
        // Task is done, removing from the list
        task_channel
            .send(TaskMessage::SendEvent("token.ready".into(), value))
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
    provider: InputParameters,
    tx: UnboundedSender<TaskMessage>,
) -> Result<bool, OAuth2Error> {
    log::trace!("cancelLogin({:?})", provider);

    let token_file = make_filename(&provider)?;
    tx.send(TaskMessage::Abort(token_file))?;
    Ok(true)
}

pub async fn request_token<I>(
    provider: InputParameters,
    interface: I,
    tx: UnboundedSender<TaskMessage>,
) -> Result<TokenKeeper, OAuth2Error>
where
    I: Interface + Send + Sync + 'static + Clone,
{
    log::trace!("requestToken({:?})", provider);

    let token_dir = interface.token_directory();
    let token_file = make_filename(&provider)?;

    let device_code_flow = DeviceCodeFlow::new(
        provider.client_id.ok_or(OAuth2Error::new(
            ErrorCodes::ParseError,
            "No Client ID supplied.".into(),
        ))?,
        provider.client_secret,
        provider.device_auth_endpoint.ok_or(OAuth2Error::new(
            ErrorCodes::ParseError,
            "No Device Auth URL supplied.".into(),
        ))?,
        provider.token_endpoint.ok_or(OAuth2Error::new(
            ErrorCodes::ParseError,
            "No Token URL supplied.".into(),
        ))?,
        tx,
    );

    let token_keeper = device_code_flow
        .get_access_token(&token_dir, &token_file, interface.clone())
        .await?;

    Ok(token_keeper)
}

pub async fn logout<I>(provider: InputParameters, interface: I) -> Result<bool, OAuth2Error>
where
    I: Interface + Send + Sync + 'static + Clone,
{
    log::trace!("logout({:?})", provider);

    let token_dir = interface.token_directory();
    let token_file = make_filename(&provider)?;

    let token_keeper = TokenKeeper::new(token_dir);

    token_keeper.delete(token_file.as_path())?;
    Ok(true)
}
