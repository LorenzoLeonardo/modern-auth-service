use std::{collections::HashMap, path::PathBuf};

use ipc_client::client::message::JsonValue;
use tokio::{
    sync::{mpsc::UnboundedReceiver, oneshot},
    task::JoinHandle,
};

use crate::interface::Interface;

pub enum TaskMessage {
    Abort(PathBuf),
    Add(PathBuf, JoinHandle<()>),
    Check(PathBuf, oneshot::Sender<bool>),
    PollingDone(PathBuf),
    SendEvent(JsonValue),
    Quit,
}

pub struct TaskManager {
    rx: UnboundedReceiver<TaskMessage>,
}

impl TaskManager {
    pub fn new(rx: UnboundedReceiver<TaskMessage>) -> Self {
        Self { rx }
    }
    pub async fn run<I: Interface + Send + Sync + 'static>(&mut self, interface: I) {
        let mut task_list = HashMap::<PathBuf, JoinHandle<()>>::new();
        loop {
            tokio::select! {
                    Some(msg) = self.rx.recv() => {
                    match msg {
                        TaskMessage::Add(key, value) => {
                            task_list.insert(key, value);
                            log::trace!("Polling tasks: {}", task_list.len());
                        }
                        TaskMessage::Abort(key) => {
                            if let Some(task) = task_list.remove(&key) {
                                task.abort();
                            }
                            log::trace!("Polling tasks: {}", task_list.len());
                        }
                        TaskMessage::Check(key, oneshot_tx) => {
                            log::trace!("Polling tasks: {}", task_list.len());
                            let existing = if let Some(task) = task_list.get(&key) {
                                log::trace!("{:?} exists, aborting task ...", task);
                                true
                            } else {
                                false
                            };
                            oneshot_tx.send(existing).unwrap_or_else(|e|{
                                log::error!("{:}", e);
                            });
                        }
                        TaskMessage::PollingDone(key) => {
                            task_list.remove(&key);
                            log::trace!("Polling tasks: {}", task_list.len());
                        }
                        TaskMessage::SendEvent(json) => {
                            interface.send_event("oauth2", json).await.unwrap_or_else(|e|{
                                log::error!("{:}", e);
                            });
                        }
                        TaskMessage::Quit => {
                            break;
                        }
                    }
                }
            }
        }
    }
}
