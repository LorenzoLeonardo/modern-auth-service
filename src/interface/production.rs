use std::path::PathBuf;

use async_trait::async_trait;
use directories::UserDirs;

use crate::oauth2::error::{ErrorCodes, OAuth2Error};

use super::Interface;

#[derive(Clone)]
pub struct Production {
    token_directory: PathBuf,
    provider_directory: PathBuf,
}

#[async_trait]
impl Interface for Production {
    fn token_directory(&self) -> PathBuf {
        self.token_directory.clone()
    }

    fn provider_directory(&self) -> PathBuf {
        self.provider_directory.clone()
    }
}

impl Production {
    pub fn new() -> Result<Self, OAuth2Error> {
        let token_directory = UserDirs::new().ok_or(OAuth2Error::new(
            ErrorCodes::DirectoryError,
            "No valid directory".to_string(),
        ))?;
        let mut token_directory = token_directory.home_dir().to_owned();

        token_directory = token_directory.join("token");

        let provider_directory = std::env::current_exe()?
            .parent()
            .ok_or(OAuth2Error::new(
                ErrorCodes::DirectoryError,
                "No valid directory".to_string(),
            ))?
            .parent()
            .ok_or(OAuth2Error::new(
                ErrorCodes::DirectoryError,
                "No valid directory".to_string(),
            ))?
            .parent()
            .ok_or(OAuth2Error::new(
                ErrorCodes::DirectoryError,
                "No valid directory".to_string(),
            ))?
            .to_path_buf();
        let provider_directory = provider_directory.join(PathBuf::from("endpoints"));

        Ok(Self {
            token_directory,
            provider_directory,
        })
    }
}
