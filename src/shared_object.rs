use std::collections::HashMap;

use async_trait::async_trait;
use ipc_client::client::message::JsonValue;
use ipc_client::client::{
    message::{CallObjectResponse, Error, OutgoingMessage},
    shared_object::SharedObject,
};

use crate::oauth2::{
    device_code_flow::{self, DeviceCodeFlowParam},
    error::{ErrorCodes, OAuth2Error},
};

pub struct DeviceCodeFlowObject;

#[async_trait]
impl SharedObject for DeviceCodeFlowObject {
    async fn remote_call(
        &self,
        method: &str,
        param: Option<HashMap<String, JsonValue>>,
    ) -> OutgoingMessage {
        log::trace!("Method: {} Param: {:?}", method, param);

        let result = async move {
            match method {
                "login" => {
                    if let Some(param) = param {
                        device_code_flow::login(DeviceCodeFlowParam::try_from(param)?).await;
                        Ok(OutgoingMessage::CallResponse(CallObjectResponse::new(
                            "This is my response from mango",
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
        .unwrap_or_else(|e| OutgoingMessage::Error(Error::new(format!("{:?}", e).as_str())));
        result
    }
}
