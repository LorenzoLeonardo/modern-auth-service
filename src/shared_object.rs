use std::collections::HashMap;

use async_trait::async_trait;
use ipc_client::client::{
    message::{CallObjectResponse, OutgoingMessage},
    shared_object::SharedObject,
};

pub struct DeviceCodeFlowObject;

#[async_trait]
impl SharedObject for DeviceCodeFlowObject {
    async fn remote_call(
        &self,
        method: &str,
        param: Option<HashMap<String, String>>,
    ) -> OutgoingMessage {
        log::trace!("Method: {} Param: {:?}", method, param);

        match method {
            "login" => {}
            "requestToken" => {}
            _ => todo!(),
        }
        OutgoingMessage::CallResponse(CallObjectResponse::new("This is my response from mango"))
    }
}
