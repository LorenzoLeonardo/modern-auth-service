mod interface;
#[allow(dead_code)]
mod oauth2;
#[allow(dead_code)]
mod shared_object;
#[allow(dead_code)]
mod task_manager;

use std::io::Write;

use chrono::Local;
use interface::production::Production;
use ipc_client::client::shared_object::ObjectDispatcher;

use log::LevelFilter;
use oauth2::error::OAuth2Result;

use shared_object::DeviceCodeFlowObject;
use task_manager::TaskManager;
use tokio::sync::mpsc::unbounded_channel;

fn init_logger(level: LevelFilter) {
    let mut log_builder = env_logger::Builder::new();
    log_builder.format(|buf, record| {
        writeln!(
            buf,
            "{}[{}:{}][{}]: {}",
            Local::now().format("[%H:%M:%S%.9f]"),
            record.target(),
            record.line().unwrap_or_default(),
            record.level(),
            record.args()
        )
    });

    log_builder.filter_level(level);
    if let Err(e) = log_builder.try_init() {
        log::error!("{:?}", e);
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> OAuth2Result<()> {
    init_logger(LevelFilter::Trace);
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
    Ok(())
}
