#[derive(Debug, Clone, Default)]
pub struct AddProjectForm {
    pub name: String,
    pub working_dir: String,
    pub visible: bool,
}

