use async_trait::async_trait;

use json_elem::jsonelem::JsonElem;
use remote_call::RemoteError;
use remote_call::SharedObject;

use tokio::sync::mpsc::UnboundedSender;

use crate::interface::Interface;
use crate::oauth2::device_code_flow::{self};
use crate::oauth2::provider::Provider;
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
    async fn remote_call(&self, method: &str, param: JsonElem) -> Result<JsonElem, RemoteError> {
        log::trace!("Method: {} Param: {:?}", method, param);

        match method {
            "login" => {
                let result = device_code_flow::login(
                    JsonElem::convert_to::<Provider>(&param)
                        .map_err(|err| RemoteError::new(JsonElem::String(err.to_string())))?,
                    self.interface.clone(),
                    self.tx.clone(),
                )
                .await
                .map(|result| JsonElem::convert_from(&result))
                .unwrap_or_else(|result| JsonElem::convert_from(&result))
                .map_err(|err| RemoteError::new(JsonElem::String(err.to_string())))?;
                Ok(result)
            }
            "cancel" => {
                let result = device_code_flow::cancel(
                    JsonElem::convert_to::<Provider>(&param)
                        .map_err(|err| RemoteError::new(JsonElem::String(err.to_string())))?,
                    self.tx.clone(),
                )
                .await
                .map(|result| JsonElem::convert_from(&result))
                .unwrap_or_else(|result| JsonElem::convert_from(&result))
                .map_err(|err| RemoteError::new(JsonElem::String(err.to_string())))?;
                Ok(result)
            }
            "requestToken" => {
                let result = device_code_flow::request_token(
                    JsonElem::convert_to::<Provider>(&param)
                        .map_err(|err| RemoteError::new(JsonElem::String(err.to_string())))?,
                    self.interface.clone(),
                )
                .await
                .map(|result| JsonElem::convert_from(&result))
                .unwrap_or_else(|result| JsonElem::convert_from(&result))
                .map_err(|err| RemoteError::new(JsonElem::String(err.to_string())))?;
                Ok(result)
            }
            "logout" => {
                let result = device_code_flow::logout(
                    JsonElem::convert_to::<Provider>(&param)
                        .map_err(|err| RemoteError::new(JsonElem::String(err.to_string())))?,
                    self.interface.clone(),
                )
                .await
                .map(|result| JsonElem::convert_from(&result))
                .unwrap_or_else(|result| JsonElem::convert_from(&result))
                .map_err(|err| RemoteError::new(JsonElem::String(err.to_string())))?;
                Ok(result)
            }
            _ => Err(RemoteError::new(JsonElem::String(format!(
                "{} method not found.",
                method
            )))),
        }
    }
}
