use std::{collections::HashMap, path::PathBuf};

use serde_json::Value;
use tokio::{
    sync::{mpsc::UnboundedReceiver, oneshot},
    task::JoinHandle,
    time::Instant,
};

use crate::interface::Interface;

const TIMEOUT: u64 = 60;

pub enum TaskMessage {
    Abort(PathBuf),
    Add(PathBuf, JoinHandle<()>),
    Check(PathBuf, oneshot::Sender<bool>),
    PollingDone(PathBuf),
    SendEvent(Value),
    ResetInactivityTimer,
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
        let timeout = std::time::Duration::from_secs(TIMEOUT);
        let mut last_activity = Instant::now();
        let mut task_list = HashMap::<PathBuf, JoinHandle<()>>::new();
        loop {
            tokio::select! {
                    Some(msg) = self.rx.recv() => {
                    match msg {
                        TaskMessage::Add(key, value) => {
                            last_activity = Instant::now();
                            task_list.insert(key, value);
                            log::trace!("Polling tasks: {}", task_list.len());
                        }
                        TaskMessage::Abort(key) => {
                            last_activity = Instant::now();
                            if let Some(task) = task_list.remove(&key) {
                                task.abort();
                            }
                            log::trace!("Polling tasks: {}", task_list.len());
                        }
                        TaskMessage::Check(key, oneshot_tx) => {
                            last_activity = Instant::now();
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
                            last_activity = Instant::now();
                            task_list.remove(&key);
                            log::trace!("Polling tasks: {}", task_list.len());
                        }
                        TaskMessage::SendEvent(json) => {
                            last_activity = Instant::now();
                            interface.send_event("oauth2.device.code.flow", "token.ready", &json).await.unwrap_or_else(|e|{
                                log::error!("{:}", e);
                            });
                        }
                        TaskMessage::ResetInactivityTimer => {
                            last_activity = Instant::now();
                            log::trace!("Activity detected, resetting inactivity timer.");
                        }
                        TaskMessage::Quit => {
                            break;
                        }
                    }
                }

                _ = tokio::time::sleep_until(last_activity + timeout) => {
                    log::warn!("No activity for {TIMEOUT} seconds, shutting down . . .");
                    log::warn!("Checking task list if there are still on going polling tasks . . .");

                    if !task_list.is_empty() {
                        log::warn!("Task list is not empty, cancel shutdown and reset inactivity timer.");
                        last_activity = Instant::now();
                    } else {
                        log::warn!("Task list is empty, exiting now!");
                        break;
                    }
                }
            }
        }
        log::info!("Task manager exited.");
    }
}
