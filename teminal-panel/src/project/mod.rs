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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Connection {
    Local,
    Ssh { service_id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshService {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: SshAuth,
}

impl SshService {
    pub fn display_destination(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }

    pub fn display_remote_location(&self, path: &std::path::Path) -> String {
        if self.port == 22 {
            format!("{}:{}", self.display_destination(), path.display())
        } else {
            format!("{}:{}:{}", self.display_destination(), self.port, path.display())
        }
    }
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

impl Project {
    pub fn new_local(name: String, working_dir: PathBuf) -> Self {
        let is_git_repo = working_dir.join(".git").exists();
        Self {
            id: Uuid::new_v4(),
            name,
            connection: Connection::Local,
            working_dir,
            is_git_repo,
        }
    }

    pub fn new_ssh(name: String, working_dir: PathBuf, service_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            connection: Connection::Ssh { service_id },
            working_dir,
            is_git_repo: false,
        }
    }
}

pub mod panel;
