pub mod curl;
pub mod reqwest;

use std::{future::Future, pin::Pin};

use oauth2::{AsyncHttpClient, HttpRequest, HttpResponse};
use strum_macros::{Display, EnumString};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    http_client::{curl::Curl, reqwest::Reqwest},
    interface::Interface,
    oauth2::error::OAuth2Error,
    task_manager::TaskMessage,
};

pub struct OAuth2Client<I>
where
    I: Interface + Clone + Send + Sync + 'static,
{
    interface: I,
    tx: Option<UnboundedSender<TaskMessage>>,
}

impl<I> OAuth2Client<I>
where
    I: Interface + Clone + Send + Sync + 'static,
{
    pub fn new(interface: I, tx: Option<UnboundedSender<TaskMessage>>) -> Self {
        Self { interface, tx }
    }
}

impl<'c, I> AsyncHttpClient<'c> for OAuth2Client<I>
where
    I: Interface + Clone + Send + Sync + 'static,
{
    type Error = OAuth2Error;

    type Future = Pin<Box<dyn Future<Output = Result<HttpResponse, Self::Error>> + Send + 'c>>;

    fn call(&'c self, request: HttpRequest) -> Self::Future {
        let interface = self.interface.clone();
        let task_message = self.tx.clone();
        Box::pin(async move {
            let result = interface.http_request(request).await?;
            if let Some(task_message) = task_message {
                let value: serde_json::Value = serde_json::from_slice(result.body())
                    .unwrap_or_else(|er| serde_json::json!({"error": er.to_string()}));
                let _ = task_message.send(TaskMessage::SendEvent("token.polling".into(), value));
            }
            Ok(result)
        })
    }
}

#[derive(Clone, Display, EnumString)]
pub enum HttpClient {
    Curl(Curl),
    Reqwest(Reqwest),
}
