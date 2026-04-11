use uuid::Uuid;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectConnectionKind {
    #[default]
    Local,
    Ssh,
}

#[derive(Debug, Clone, Default)]
pub struct AddProjectForm {
    pub name: String,
    pub selected_dir: Option<PathBuf>,
    pub connection_kind: ProjectConnectionKind,
    pub ssh_service_id: Option<Uuid>,
    pub visible: bool,
}
