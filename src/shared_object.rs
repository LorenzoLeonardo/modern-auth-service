use async_trait::async_trait;

use ipc_client::client::error::Error;
use ipc_client::client::message::JsonValue;
use ipc_client::client::shared_object::SharedObject;

use tokio::sync::mpsc::UnboundedSender;

use crate::interface::Interface;
use crate::oauth2::provider::Provider;
use crate::oauth2::{
    device_code_flow::{self},
    error::{ErrorCodes, OAuth2Error},
};
use crate::task_manager::TaskMessage;

pub struct DeviceCodeFlowObject<I>
where
    I: Interface + Send + Sync + 'static,
{
    interface: I,
    tx: UnboundedSender<TaskMessage>,
}

impl<I> DeviceCodeFlowObject<I>
where
    I: Interface + Send + Sync + 'static,
{
    pub fn new(interface: I, tx: UnboundedSender<TaskMessage>) -> Self {
        Self { interface, tx }
    }
}

#[async_trait]
impl<I> SharedObject for DeviceCodeFlowObject<I>
where
    I: Interface + Send + Sync + 'static + Clone,
{
    async fn remote_call(
        &self,
        method: &str,
        param: Option<JsonValue>,
    ) -> Result<JsonValue, Error> {
        log::trace!("Method: {} Param: {:?}", method, param);

        async move {
            match method {
                "login" => {
                    if let Some(param) = param {
                        let result = device_code_flow::login(
                            Provider::try_from(param)?,
                            self.interface.clone(),
                            self.tx.clone(),
                        )
                        .await?;

                        Ok(JsonValue::try_from(result)?)
                    } else {
                        Err(OAuth2Error::new(
                            ErrorCodes::InvalidParameters,
                            String::from("No parameter"),
                        ))
                    }
                }
                "cancel" => {
                    if let Some(param) = param {
                        let result =
                            device_code_flow::cancel(Provider::try_from(param)?, self.tx.clone())
                                .await?;

                        Ok(JsonValue::try_from(result)?)
                    } else {
                        Err(OAuth2Error::new(
                            ErrorCodes::InvalidParameters,
                            String::from("No parameter"),
                        ))
                    }
                }
                "requestToken" => {
                    if let Some(param) = param {
                        let result = device_code_flow::request_token(
                            Provider::try_from(param)?,
                            self.interface.clone(),
                        )
                        .await?;

                        Ok(JsonValue::try_from(result)?)
                    } else {
                        Err(OAuth2Error::new(
                            ErrorCodes::InvalidParameters,
                            String::from("No parameter"),
                        ))
                    }
                }
                "logout" => {
                    if let Some(param) = param {
                        let result = device_code_flow::logout(
                            Provider::try_from(param)?,
                            self.interface.clone(),
                        )
                        .await?;

                        Ok(JsonValue::try_from(result)?)
                    } else {
                        Err(OAuth2Error::new(
                            ErrorCodes::InvalidParameters,
                            String::from("No parameter"),
                        ))
                    }
                }
                _ => todo!(),
            }
        }
        .await
        .map_err(|e| Error::new(JsonValue::String(e.to_string())))
    }
}
