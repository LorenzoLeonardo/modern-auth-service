use async_trait::async_trait;

use ipc_client::client::message::JsonValue;
use ipc_client::client::{
    message::{CallObjectResponse, Error, OutgoingMessage},
    shared_object::SharedObject,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::interface::Interface;
use crate::oauth2::{
    device_code_flow::{self, DeviceCodeFlowParam},
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
    async fn remote_call(&self, method: &str, param: Option<JsonValue>) -> OutgoingMessage {
        log::trace!("Method: {} Param: {:?}", method, param);

        async move {
            match method {
                "login" => {
                    if let Some(param) = param {
                        let result = device_code_flow::login(
                            DeviceCodeFlowParam::try_from(param)?,
                            self.interface.clone(),
                            self.tx.clone(),
                        )
                        .await?;

                        Ok(OutgoingMessage::CallResponse(CallObjectResponse::new(
                            JsonValue::try_from(result)?,
                        )))
                    } else {
                        Err(OAuth2Error::new(
                            ErrorCodes::InvalidParameters,
                            String::from("No parameter"),
                        ))
                    }
                }
                "cancel" => {
                    if let Some(param) = param {
                        let result = device_code_flow::cancel(
                            DeviceCodeFlowParam::try_from(param)?,
                            self.tx.clone(),
                        )
                        .await?;

                        Ok(OutgoingMessage::CallResponse(CallObjectResponse::new(
                            JsonValue::try_from(result)?,
                        )))
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
                            DeviceCodeFlowParam::try_from(param)?,
                            self.interface.clone(),
                        )
                        .await?;

                        Ok(OutgoingMessage::CallResponse(CallObjectResponse::new(
                            JsonValue::try_from(result)?,
                        )))
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
                            DeviceCodeFlowParam::try_from(param)?,
                            self.interface.clone(),
                        )
                        .await?;

                        Ok(OutgoingMessage::CallResponse(CallObjectResponse::new(
                            JsonValue::try_from(result)?,
                        )))
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
        .unwrap_or_else(|e| {
            let err_stream = serde_json::to_string(&e).unwrap_or_else(|e| e.to_string());
            OutgoingMessage::Error(Error::new(err_stream.as_str()))
        })
    }
}
