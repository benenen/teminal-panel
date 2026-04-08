pub mod model;
pub mod pty;
pub mod render;
pub mod subscription;

use uuid::Uuid;

pub struct TerminalState {
    pub id: Uuid,
    pub project_id: Uuid,
    pub model: model::TerminalModel,
    pub input_buf: String,
    pub writer: Box<dyn std::io::Write + Send>,
    pub lifecycle: Option<pty::PtyLifecycle>,
    pub last_size: Option<model::TerminalSize>,
    pub resize: Box<dyn Fn(model::TerminalSize) -> pty::PtyResult<()> + Send>,
}
