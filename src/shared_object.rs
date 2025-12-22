use async_trait::async_trait;

use ipc_broker::worker::SharedObject;
use json_result::r#struct::JsonResult;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

use crate::interface::Interface;
use crate::oauth2::device_code_flow::{self};
use crate::oauth2::error::{ErrorCodes, OAuth2Error};
use crate::oauth2::provider::Provider;
use crate::openid::{self, ApplicationNonce};
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
    async fn call(&self, method: &str, args: &Value) -> Value {
        log::trace!("Method: {} Param: {:?}", method, args);

        let param: Provider = match serde_json::from_value(args.clone()) {
            Ok(p) => p,
            Err(e) => {
                let e = OAuth2Error::from(e);
                return JsonResult::<(), OAuth2Error>(Err(e)).into();
            }
        };

        match method {
            "login" => {
                let result =
                    device_code_flow::login(param, self.interface.clone(), self.tx.clone()).await;
                JsonResult::from(result).into()
            }
            "cancel" => {
                let result = device_code_flow::cancel(param, self.tx.clone()).await;
                JsonResult::from(result).into()
            }
            "requestToken" => {
                let result = device_code_flow::request_token(param, self.interface.clone()).await;
                JsonResult::from(result).into()
            }
            "logout" => {
                let result = device_code_flow::logout(param, self.interface.clone()).await;
                JsonResult::from(result).into()
            }
            "verifyIDToken" => {
                let result =
                    openid::verify_id_token(param, ApplicationNonce::new(), self.interface.clone())
                        .await;
                JsonResult::from(result).into()
            }
            _ => {
                let e = OAuth2Error::new(
                    ErrorCodes::OtherError,
                    format!("{} method not found.", method),
                );
                JsonResult::<(), OAuth2Error>(Err(e)).into()
            }
        }
    }
}
