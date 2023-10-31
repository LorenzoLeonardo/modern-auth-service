mod oauth2;
mod shared_object;

use std::io::Write;
use std::str::FromStr;

use chrono::Local;
use ipc_client::client::shared_object::ObjectDispatcher;

use log::LevelFilter;
use oauth2::error::OAuth2Result;

use shared_object::DeviceCodeFlowObject;

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
    let mut shared = ObjectDispatcher::new().await.unwrap();

    shared
        .register_object("oauth2.device.code.flow", Box::new(DeviceCodeFlowObject))
        .await
        .unwrap();

    let _r = shared.spawn().await;
    Ok(())
}
