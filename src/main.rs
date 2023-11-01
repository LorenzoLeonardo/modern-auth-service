mod interface;
#[allow(dead_code)]
mod oauth2;
#[allow(dead_code)]
mod shared_object;
#[allow(dead_code)]
mod task_manager;

use std::io::Write;
use std::str::FromStr;

use chrono::Local;
use interface::production::Production;
use ipc_client::client::shared_object::ObjectDispatcher;

use log::LevelFilter;
use oauth2::curl::Curl;
use oauth2::error::OAuth2Result;

use shared_object::DeviceCodeFlowObject;
use task_manager::TaskManager;
use tokio::sync::mpsc::unbounded_channel;

fn init_logger(level: &str) {
    let mut log_builder = env_logger::Builder::new();
    log_builder.format(|buf, record| {
        let mut module = "";
        if let Some(path) = record.module_path() {
            if let Some(split) = path.split("::").last() {
                module = split;
            }
        }

        writeln!(
            buf,
            "{}[{}]:{}: {}",
            Local::now().format("[%d-%m-%Y %H:%M:%S]"),
            record.level(),
            module,
            record.args()
        )
    });

    log_builder.filter_level(LevelFilter::from_str(level).unwrap_or(LevelFilter::Info));
    if let Err(e) = log_builder.try_init() {
        log::error!("{:?}", e);
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> OAuth2Result<()> {
    init_logger("trace");
    let (tx, rx) = unbounded_channel();

    let mut shared = ObjectDispatcher::new().await.unwrap();
    let interface = Production::new()?;
    let curl = Curl::new();
    let object = DeviceCodeFlowObject::new(interface, curl, tx);

    shared
        .register_object("oauth2.device.code.flow", Box::new(object))
        .await
        .unwrap();

    let _r = shared.spawn().await;

    let mut task = TaskManager::new(rx);

    task.run().await;
    Ok(())
}
