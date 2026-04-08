#[derive(Debug, Clone, Default)]
pub struct AddAgentForm {
    pub name: String,
    pub working_dir: String,
    pub visible: bool,
}
