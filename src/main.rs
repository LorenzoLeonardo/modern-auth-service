mod interface;
#[allow(dead_code)]
mod oauth2;
#[allow(dead_code)]
mod shared_object;
#[allow(dead_code)]
mod task_manager;

use interface::production::Production;
use remote_call::{logger::ENV_LOGGER, Connector, Error, SharedObjectDispatcher};

use oauth2::error::{OAuth2Error, OAuth2Result};

use shared_object::DeviceCodeFlowObject;
use task_manager::TaskManager;
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedSender},
    task::JoinHandle,
};

use crate::task_manager::TaskMessage;

pub fn setup_logger() {
    let level = std::env::var(ENV_LOGGER)
        .map(|var| match var.to_lowercase().as_str() {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            "off" => log::LevelFilter::Off,
            _ => log::LevelFilter::Info,
        })
        .unwrap_or_else(|_| log::LevelFilter::Info);

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
    setup_logger();

    let version = env!("CARGO_PKG_VERSION");

    log::info!("Starting modern-auth-service v.{}", version);

    let (tx, rx) = unbounded_channel();

    match initialize().await {
        Ok((mut shared, interface)) => {
            let object = DeviceCodeFlowObject::new(interface.clone(), tx.clone());

            shared
                .register_object("oauth2.device.code.flow", Box::new(object))
                .await
                .unwrap();

            // Handle the spawned SharedObject, if something happens to the server, we must exist gracefully.
            let spawn = shared.spawn().await;
            handle_spawn_result(spawn, tx).await;

            let mut task = TaskManager::new(rx);

            task.run(interface).await;
        }
        Err(err) => {
            log::error!("{}", err.to_string());
        }
    }

    log::info!("Stopping modern-auth-service v.{}", version);

    Ok(())
}

async fn initialize() -> Result<(SharedObjectDispatcher, Production), OAuth2Error> {
    let shared = SharedObjectDispatcher::new().await?;
    let connector = Connector::connect().await?;
    let interface = Production::new(connector)?;
    Ok((shared, interface))
}

async fn handle_spawn_result(
    spawn: JoinHandle<Result<(), Error>>,
    tx: UnboundedSender<TaskMessage>,
) {
    tokio::spawn(async move {
        match spawn.await {
            Ok(result) => {
                if let Err(err) = result {
                    log::error!("{}", err.to_string());
                    tx.send(TaskMessage::Quit).unwrap_or_else(|err| {
                        log::error!("{}", err.to_string());
                    });
                }
            }
            Err(err) => {
                log::error!("{}", err.to_string());
                tx.send(TaskMessage::Quit).unwrap_or_else(|err| {
                    log::error!("{}", err.to_string());
                });
            }
        }
    });
}
