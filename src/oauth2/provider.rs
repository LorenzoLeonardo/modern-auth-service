use oauth2::{url::Url, AuthUrl, ClientId, ClientSecret, DeviceAuthorizationUrl, Scope, TokenUrl};
use openidconnect::core::CoreIdToken;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Hash, Default, Clone)]
pub struct SmtpHostName(pub String);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SmtpPort(pub u16);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProfileUrl(pub Url);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InputParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_endpoint: Option<AuthUrl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_endpoint: Option<TokenUrl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_auth_endpoint: Option<DeviceAuthorizationUrl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<Scope>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<ClientId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<ClientSecret>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<CoreIdToken>,
}
