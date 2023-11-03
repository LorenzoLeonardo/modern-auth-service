use oauth2::{url::Url, AuthUrl, ClientId, ClientSecret, DeviceAuthorizationUrl, Scope, TokenUrl};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Hash, Default, Clone)]
pub struct SmtpHostName(pub String);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SmtpPort(pub u16);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProfileUrl(pub Url);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Provider {
    pub process: String,
    pub provider: String,
    pub authorization_endpoint: AuthUrl,
    pub token_endpoint: TokenUrl,
    pub device_auth_endpoint: DeviceAuthorizationUrl,
    pub scopes: Vec<Scope>,
    pub client_id: ClientId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<ClientSecret>,
}
