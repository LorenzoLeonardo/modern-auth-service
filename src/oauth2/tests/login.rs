use std::time::Duration;

use crate::oauth2::device_code_flow::login;
use crate::oauth2::provider::InputParameters;
use crate::task_manager::{TaskManager, TaskMessage};
use crate::{interface::mock::Mock, setup_logger};

use http::{HeaderMap, HeaderValue, Response, StatusCode};
use oauth2::{url::Url, AuthUrl, ClientId, DeviceAuthorizationUrl, Scope, TokenUrl};
use tokio::sync::mpsc::unbounded_channel;

fn build_mock_provider() -> InputParameters {
    InputParameters {
        authorization_endpoint: Some(AuthUrl::from_url(
            Url::parse("https://login.microsoftonline.com/common/oauth2/v2.0/authorize").unwrap(),
        )),
        token_endpoint: Some(TokenUrl::from_url(
            Url::parse("https://login.microsoftonline.com/common/oauth2/v2.0/token").unwrap(),
        )),
        device_auth_endpoint: Some(DeviceAuthorizationUrl::from_url(
            Url::parse("https://login.microsoftonline.com/common/oauth2/v2.0/devicecode").unwrap(),
        )),
        scopes: Some(vec![
            Scope::new("offline_access".into()),
            Scope::new("https://outlook.office.com/SMTP.Send".into()),
            Scope::new("https://outlook.office.com/User.Read".into()),
        ]),
        client_id: Some(ClientId::new("64c5d510-4b7e-4a18-8869-89778461c266".into())),
        client_secret: None,
        process: String::from("Process Name"),
        provider: String::from("Microsoft"),
        id_token: None,
    }
}

#[tokio::test]
async fn test_login() {
    setup_logger();
    let (tx, rx) = unbounded_channel();
    let interface = Mock::new();
    let mut inner = interface.clone();
    tokio::spawn(async move {
        let body = r#"{"user_code":"usercode-123","device_code":"devicecode-123","verification_uri":"https://verification_url","expires_in":20,"interval":1,"message":"Mock message"}"#.as_bytes().to_vec();
        let mut headers = HeaderMap::new();

        headers.insert(
            "content-type",
            HeaderValue::from_static("application/json; charset=utf-8"),
        );

        let response = Response::new(body);
        let (mut parts, body) = response.into_parts();

        parts.status = StatusCode::OK;
        let response = Response::from_parts(parts, body);

        inner = inner.set_mock_response(response);
        let provider = build_mock_provider();
        let result = login(provider, inner, tx.clone()).await.unwrap();

        assert_eq!(result.device_code().secret(), "devicecode-123");
        assert_eq!(result.expires_in(), Duration::from_secs(20));
        assert_eq!(result.user_code().secret(), "usercode-123");
        assert_eq!(
            result.verification_uri().as_str(),
            "https://verification_url"
        );
        assert_eq!(result.interval(), Duration::from_secs(1));

        log::trace!("Result: {:?}", result);

        tokio::time::sleep(Duration::from_millis(1)).await;
        tx.send(TaskMessage::Quit).unwrap();
    });

    TaskManager::new(rx).run(interface).await;
}
