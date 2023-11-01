pub mod mock;
pub mod production;

use std::path::PathBuf;

use async_trait::async_trait;

#[async_trait]
pub trait Interface {
    fn token_directory(&self) -> PathBuf;
    fn provider_directory(&self) -> PathBuf;
}
