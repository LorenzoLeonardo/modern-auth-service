mod interface;
#[allow(dead_code)]
mod oauth2;
#[allow(dead_code)]
mod shared_object;
#[allow(dead_code)]
mod task_manager;

use interface::production::Production;
use ipc_client::client::shared_object::ObjectDispatcher;

use log::LevelFilter;
use oauth2::error::OAuth2Result;

use shared_object::DeviceCodeFlowObject;
use task_manager::TaskManager;
use tokio::sync::mpsc::unbounded_channel;

pub fn setup_logger(level: LevelFilter) {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}:{}]: {}",
                chrono::Local::now().format("%H:%M:%S%.9f"),
                record.level(),
                record.target(),
                record.line().unwrap_or(0),
                message
            ))
        })
        .level(level)
        .chain(std::io::stdout())
        .apply()
        .unwrap_or_else(|e| {
            eprintln!("{:?}", e);
        });
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> OAuth2Result<()> {
    setup_logger(LevelFilter::Trace);

    let version = env!("CARGO_PKG_VERSION");

    log::info!("Starting modern-auth-service v.{}", version);

    let (tx, rx) = unbounded_channel();

    let mut shared = ObjectDispatcher::new().await.unwrap();
    let interface = Production::new()?;

    let object = DeviceCodeFlowObject::new(interface, tx);

    shared
        .register_object("oauth2.device.code.flow", Box::new(object))
        .await
        .unwrap();

    let _r = shared.spawn().await;

    let mut task = TaskManager::new(rx);

    task.run().await;

    log::info!("Stopping modern-auth-service v.{}", version);
    Ok(())
}
