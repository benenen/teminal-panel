use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct AddProjectForm {
    pub name: String,
    pub selected_dir: Option<PathBuf>,
    pub visible: bool,
}

