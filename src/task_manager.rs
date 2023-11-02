use std::{collections::HashMap, path::PathBuf};

use tokio::{
    sync::{mpsc::UnboundedReceiver, oneshot},
    task::JoinHandle,
};

pub enum TaskMessage {
    Abort(PathBuf),
    Add(PathBuf, JoinHandle<()>),
    Check(PathBuf, oneshot::Sender<bool>),
}

pub struct TaskManager {
    rx: UnboundedReceiver<TaskMessage>,
}

impl TaskManager {
    pub fn new(rx: UnboundedReceiver<TaskMessage>) -> Self {
        Self { rx }
    }
    pub async fn run(&mut self) {
        let mut task_list = HashMap::<PathBuf, JoinHandle<()>>::new();
        loop {
            tokio::select! {
                    Some(msg) = self.rx.recv() => {
                    match msg {
                        TaskMessage::Add(key, value) => {
                            task_list.insert(key, value);
                        }
                        TaskMessage::Abort(key) => {
                            if let Some(task) = task_list.remove(&key) {
                                task.abort();
                            }
                        }
                        TaskMessage::Check(key, oneshot_tx) => {
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
                    }
                }
            }
        }
    }
}
