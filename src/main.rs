mod http_client;
mod interface;
mod logger;
#[allow(dead_code)]
mod oauth2;
mod openid;
#[allow(dead_code)]
mod shared_object;
#[allow(dead_code)]
mod task_manager;

use interface::production::Production;

use ipc_broker::{client::IPCClient, worker::WorkerBuilder};
use oauth2::error::OAuth2Result;

use shared_object::DeviceCodeFlowObject;
use task_manager::TaskManager;
use tokio::sync::mpsc::unbounded_channel;

use crate::{
    http_client::{curl::Curl, HttpClient},
    task_manager::TaskMessage,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> OAuth2Result<()> {
    logger::setup_logger();

    let version = env!("CARGO_PKG_VERSION");

    log::info!("Starting modern-auth-service v.{}", version);

    let (tx, rx) = unbounded_channel();
    let connector = IPCClient::connect().await?;
    let http_client = HttpClient::Curl(Curl::default());
    let interface = Production::new(connector, http_client)?;
    let object = DeviceCodeFlowObject::new(interface.clone(), tx.clone());

    let (builder, shutdown) = WorkerBuilder::new()
        .add("oauth2.device.code.flow", object)
        .with_graceful_shutdown();

    let handle = tokio::spawn(async move { builder.spawn().await });

    let task_handle = tokio::spawn(async move {
        let mut task = TaskManager::new(rx);

        task.run(interface).await;
        let _ = shutdown.send(true);
    });

    handle.await??;

    let _ = tx.send(TaskMessage::Quit);
    let _ = task_handle.await;
    log::info!("Stopping modern-auth-service v.{}", version);

    Ok(())
}
