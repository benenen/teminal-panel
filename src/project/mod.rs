use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub connection: Connection,
    pub working_dir: PathBuf,
    pub is_git_repo: bool,
    #[serde(skip)]
    pub status: ProjectStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Connection {
    Local,
    Ssh {
        host: String,
        port: u16,
        user: String,
        auth: SshAuth,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SshAuth {
    Password(String),
    Key {
        path: PathBuf,
        passphrase: Option<String>,
    },
    Agent,
}

#[derive(Debug, Clone, Default)]
pub enum ProjectStatus {
    #[default]
    Disconnected,
    Connected,
    Connecting,
    Error(String),
}

impl Project {
    pub fn new_local(name: String, working_dir: PathBuf) -> Self {
        let is_git_repo = working_dir.join(".git").exists();
        Self {
            id: Uuid::new_v4(),
            name,
            connection: Connection::Local,
            working_dir,
            is_git_repo,
            status: ProjectStatus::Disconnected,
        }
    }
}

pub mod panel;
