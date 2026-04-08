pub mod pty;
pub mod subscription;

use uuid::Uuid;

pub struct TerminalState {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub output: String,
    pub input_buf: String,
    pub pending_bytes: Vec<u8>,
    pub writer: Box<dyn std::io::Write + Send>,
    pub lifecycle: Option<pty::PtyLifecycle>,
}
