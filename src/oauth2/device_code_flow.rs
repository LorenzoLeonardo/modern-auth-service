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
use tokio::sync::mpsc::UnboundedSender;

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

impl TryFrom<HashMap<String, JsonValue>> for DeviceCodeFlowParam {
    type Error = OAuth2Error;

    fn try_from(value: HashMap<String, JsonValue>) -> Result<Self, Self::Error> {
        #[derive(Serialize, Deserialize)]
        #[serde(transparent)]
        struct MyData {
            value: HashMap<String, JsonValue>,
        }
        let value = MyData { value };
        let value = serde_json::to_vec(&value)?;
        let value: DeviceCodeFlowParam = serde_json::from_slice(value.as_slice())?;
        Ok(value)
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

fn make_filename(param: &DeviceCodeFlowParam) -> PathBuf {
    PathBuf::from(param.to_string())
}

pub async fn login<I>(
    param: DeviceCodeFlowParam,
    interface: I,
    tx: UnboundedSender<TaskMessage>,
) -> Result<StandardDeviceAuthorizationResponse, OAuth2Error>
where
    I: Interface + Send + Sync + 'static + Clone,
{
    log::trace!("Login Method(): {:?}", param);

    let provider_dir = interface.provider_directory();
    let token_dir = interface.token_directory();
    let token_file = make_filename(&param);

    log::trace!("Provider Directory: {:?}", provider_dir);
    log::trace!("Token Directory: {:?}", token_dir);
    log::trace!("Token File: {:?}", token_file);

    let provider = Provider::read(
        provider_dir.as_path(),
        &PathBuf::from(param.provider.as_str()),
    )?;

    let device_code_flow = DeviceCodeFlow::new(
        provider.client_id,
        provider.client_secret,
        provider.device_auth_endpoint,
        provider.token_endpoint,
    );

    let scope_vec: Vec<Scope> = param
        .scopes
        .iter()
        .map(|s| Scope::new(s.to_string()))
        .collect();

    let device_auth_response = device_code_flow
        .request_device_code(scope_vec, |request| async {
            interface.http_request(request).await
        })
        .await?;

    let result = device_auth_response.clone();
    let token_file_clone = token_file.clone();
    // Start polling at the background
    let handle = tokio::spawn(async move {
        let token = device_code_flow
            .poll_access_token(device_auth_response, |request| async {
                interface.http_request(request).await
            })
            .await
            .unwrap();
        let mut token_keeper = TokenKeeper::from(token);
        token_keeper.set_directory(token_dir);

        token_keeper.save(&token_file_clone).unwrap();
    });
    // Send this polling task to the background
    tx.send(TaskMessage::AddTask(token_file, handle)).unwrap();

    Ok(result)
}

pub async fn cancel(
    param: DeviceCodeFlowParam,
    tx: UnboundedSender<TaskMessage>,
) -> Result<bool, OAuth2Error> {
    let token_file = make_filename(&param);
    tx.send(TaskMessage::AbortTask(token_file))?;
    Ok(true)
}

pub async fn request_token<I>(
    param: DeviceCodeFlowParam,
    interface: I,
) -> Result<TokenKeeper, OAuth2Error>
where
    I: Interface + Send + Sync + 'static + Clone,
{
    log::trace!("Request Token Method(): {:?}", param);

    let provider_dir = interface.provider_directory();
    let token_dir = interface.token_directory();
    let token_file = make_filename(&param);

    log::trace!("Provider Directory: {:?}", provider_dir);
    log::trace!("Token Directory: {:?}", token_dir);
    log::trace!("Token File: {:?}", token_file);

    let provider = Provider::read(
        provider_dir.as_path(),
        &PathBuf::from(param.provider.as_str()),
    )?;

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

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use oauth2::{url::Url, AuthUrl, ClientId, DeviceAuthorizationUrl, Scope, TokenUrl};
    use tokio::sync::mpsc::unbounded_channel;

    use crate::{
        init_logger,
        interface::mock::Mock,
        oauth2::provider::{ProfileUrl, Provider, SmtpHostName, SmtpPort},
    };

    use super::{login, DeviceCodeFlowParam};

    fn build_mock_provider() -> Provider {
        Provider {
            authorization_endpoint: AuthUrl::from_url(
                Url::parse("https://login.microsoftonline.com/common/oauth2/v2.0/authorize")
                    .unwrap(),
            ),
            token_endpoint: TokenUrl::from_url(
                Url::parse("https://login.microsoftonline.com/common/oauth2/v2.0/token").unwrap(),
            ),
            device_auth_endpoint: DeviceAuthorizationUrl::from_url(
                Url::parse("https://login.microsoftonline.com/common/oauth2/v2.0/devicecode")
                    .unwrap(),
            ),
            scopes: vec![
                Scope::new("offline_access".into()),
                Scope::new("https://outlook.office.com/SMTP.Send".into()),
                Scope::new("https://outlook.office.com/User.Read".into()),
            ],
            smtp_server: SmtpHostName("smtp.office365.com".into()),
            smtp_server_port: SmtpPort(587),
            profile_endpoint: ProfileUrl(
                Url::parse("https://outlook.office.com/api/v2.0/me").unwrap(),
            ),
            client_id: ClientId::new("64c5d510-4b7e-4a18-8869-89778461c266".into()),
            client_secret: None,
        }
    }

    #[tokio::test]
    async fn test_login() {
        init_logger("trace");
        let (tx, _rx) = unbounded_channel();
        let param = DeviceCodeFlowParam::new("mock_process".into(), "Microsoft".into(), Vec::new());
        let provider_file = PathBuf::from_str(param.provider.as_str()).unwrap();
        let interface = Mock::new(build_mock_provider(), provider_file);

        let result = login(param, interface, tx).await;

        log::trace!("{:?}", result);
    }
}
