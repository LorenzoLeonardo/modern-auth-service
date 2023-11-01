use std::{collections::HashMap, path::PathBuf};

use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};

pub enum TaskMessage {
    AddTask(PathBuf, JoinHandle<()>),
    AbortTask(PathBuf),
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
                        TaskMessage::AddTask(key, value) => {
                            task_list.insert(key, value);
                        }
                        TaskMessage::AbortTask(key) => {
                            if let Some(task) = task_list.remove(&key) {
                                task.abort();
                            }
                        }
                    }
                }
            }
        }
    }
}
