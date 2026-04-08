use crate::project::{panel::AddProjectForm, Project};
use crate::config::AppConfig;
use crate::terminal::{model::TerminalModel, render, subscription, TerminalState};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length, Task, Theme};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use tokio::sync::mpsc;
use uuid::Uuid;

const PTY_CHANNEL_CAPACITY: usize = 256;

pub struct App {
    pub config: AppConfig,
    pub selected_project: Option<Uuid>,
    pub add_form: AddProjectForm,
    pub terminals: HashMap<Uuid, TerminalState>,
    pub pty_tx: mpsc::Sender<(Uuid, Vec<u8>)>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectProject(Uuid),
    AddProject { name: String, working_dir: String },
    RemoveProject(Uuid),
    ProjectStatusChanged(Uuid, String),
    ShowAddProjectForm,
    HideAddProjectForm,
    FormNameChanged(String),
    FormDirChanged(String),
    ChooseProjectFolder,
    ProjectFolderSelected(Option<PathBuf>),
    SubmitAddProjectForm,
    OpenTerminal(Uuid),
    PtyOutput(Uuid, Vec<u8>),
    TerminalViewportChanged(Uuid, TerminalViewport),
    TerminalInput(Uuid, String),
    InputChanged(Uuid, String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalViewport {
    pub width: f32,
    pub height: f32,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let config = AppConfig::load();
        let (pty_tx, pty_rx) = mpsc::channel(PTY_CHANNEL_CAPACITY);
        subscription::install_receiver(pty_rx);

        (
            Self {
                config,
                selected_project: None,
                add_form: Default::default(),
                terminals: HashMap::new(),
                pty_tx,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectProject(id) => {
                self.selected_project = Some(id);
            }
            Message::AddProject { name, working_dir } => {
                self.add_local_project(name, PathBuf::from(working_dir));
            }
            Message::RemoveProject(id) => {
                self.config.projects.retain(|project| project.id != id);

                if let Some(mut terminal) = self.terminals.remove(&id) {
                    Self::shutdown_terminal(&mut terminal);
                }

                if self.selected_project == Some(id) {
                    self.selected_project = None;
                }

                self.config.save();
            }
            Message::ProjectStatusChanged(_, _) => {}
            Message::ShowAddProjectForm => {
                self.add_form.visible = true;
            }
            Message::HideAddProjectForm => {
                self.add_form = Default::default();
            }
            Message::FormNameChanged(value) => {
                self.add_form.name = value;
            }
            Message::FormDirChanged(_value) => {
                // Keep for backward compatibility, but unused in new UI
            }
            Message::ChooseProjectFolder => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .pick_folder()
                            .await
                            .map(|handle| handle.path().to_path_buf())
                    },
                    Message::ProjectFolderSelected,
                );
            }
            Message::ProjectFolderSelected(selection) => {
                if let Some(path) = selection {
                    self.add_form.selected_dir = Some(path);
                }
            }
            Message::SubmitAddProjectForm => {
                if let Some(path) = self.add_form.selected_dir.clone() {
                    if self.add_local_project(self.add_form.name.clone(), path) {
                        self.add_form = Default::default();
                    }
                }
            }
            Message::OpenTerminal(project_id) => {
                if self.terminals.contains_key(&project_id) {
                    return Task::none();
                }

                if let Some(project) = self
                    .config
                    .projects
                    .iter()
                    .find(|project| project.id == project_id)
                {
                    match crate::terminal::pty::spawn_shell(
                        &project.working_dir,
                        self.pty_tx.clone(),
                        project_id,
                    ) {
                        Ok(handle) => {
                            let crate::terminal::pty::PtyHandle {
                                writer,
                                lifecycle,
                                controller,
                            } = handle;

                            self.terminals.insert(
                                project_id,
                                TerminalState {
                                    id: project_id,
                                    project_id: project_id,
                                    model: TerminalModel::new(120, 40),
                                    input_buf: String::new(),
                                    writer,
                                    lifecycle: Some(lifecycle),
                                    last_size: None,
                                    resize: Box::new(move |size| {
                                        controller.resize(portable_pty::PtySize {
                                            rows: size.rows,
                                            cols: size.cols,
                                            pixel_width: 0,
                                            pixel_height: 0,
                                        })?;
                                        Ok(())
                                    }),
                                },
                            );
                        }
                        Err(error) => {
                            eprintln!("Failed to spawn PTY: {error}");
                        }
                    }
                }
            }
            Message::PtyOutput(id, bytes) => {
                if let Some(terminal) = self.terminals.get_mut(&id) {
                    terminal.model.advance_bytes(&bytes);
                }
            }
            Message::TerminalViewportChanged(id, viewport) => {
                if let Some(terminal) = self.terminals.get_mut(&id) {
                    let Some(size) = terminal_size_for_viewport(viewport) else {
                        return Task::none();
                    };

                    if terminal.last_size == Some(size) {
                        return Task::none();
                    }

                    terminal.model.resize(size);

                    if let Err(error) = (terminal.resize)(size) {
                        eprintln!("Failed to resize PTY for terminal {id}: {error}");
                        return Task::none();
                    }

                    terminal.last_size = Some(size);
                }
            }
            Message::TerminalInput(id, input) => {
                if let Some(terminal) = self.terminals.get_mut(&id) {
                    let _ = terminal.writer.write_all(input.as_bytes());
                    terminal.input_buf.clear();
                }
            }
            Message::InputChanged(id, value) => {
                if let Some(terminal) = self.terminals.get_mut(&id) {
                    terminal.input_buf = value;
                }
            }
        }

        Task::none()
    }

    fn add_local_project(&mut self, name: String, working_dir: PathBuf) -> bool {
        let name = name.trim().to_string();

        if name.is_empty() || !working_dir.is_dir() {
            return false;
        }

        self.config.projects.push(Project::new_local(name, working_dir));
        self.config.save();
        true
    }

    fn shutdown_terminal(terminal: &mut TerminalState) {
        if let Some(lifecycle) = terminal.lifecycle.as_mut() {
            lifecycle.shutdown();
        }
        terminal.lifecycle = None;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let main_content = row![self.view_project_panel(), self.view_terminal_area()]
            .spacing(16)
            .padding(16);

        if self.add_form.visible {
            let selected_dir = self
                .add_form
                .selected_dir
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "No folder selected".into());

            let modal = container(
                column![
                    text("Add Project").size(20),
                    text_input("Project Name", &self.add_form.name)
                        .on_input(Message::FormNameChanged)
                        .on_submit(Message::SubmitAddProjectForm),
                    row![
                        text(selected_dir).size(12),
                        button(text("Choose Folder"))
                            .on_press(Message::ChooseProjectFolder),
                    ]
                    .spacing(8)
                    .align_y(iced::alignment::Vertical::Center),
                    row![
                        button(text("Add"))
                            .width(Length::Fill)
                            .on_press(Message::SubmitAddProjectForm),
                        button(text("Cancel"))
                            .width(Length::Fill)
                            .on_press(Message::HideAddProjectForm),
                    ]
                    .spacing(8),
                ]
                .spacing(16)
                .padding(20)
            )
            .width(Length::Fixed(400.0))
            .style(|_| {
                container::Style::default()
                    .background(iced::Color::from_rgb8(45, 45, 45))
                    .border(iced::Border {
                        color: iced::Color::from_rgb8(100, 100, 100),
                        width: 1.0,
                        radius: 8.0.into(),
                    })
            });

            let overlay = container(modal)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| {
                    container::Style::default()
                        .background(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5))
                })
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            column![
                container(main_content)
                    .width(Length::Fill)
                    .height(Length::Fill),
                overlay,
            ]
            .into()
        } else {
            main_content.into()
        }
    }

    fn view_project_panel(&self) -> Element<'_, Message> {
        let project_list = self.config.projects.iter().fold(column![], |column, project| {
            let details = column![
                text(&project.name).size(16),
                text(project.working_dir.display().to_string()).size(12)
            ]
            .spacing(2)
            .width(Length::Fill);

            column.push(
                row![
                    button(details)
                        .width(Length::Fill)
                        .on_press(Message::SelectProject(project.id)),
                    button(text("x")).on_press(Message::RemoveProject(project.id)),
                ]
                .spacing(6),
            )
        });

        container(
            column![
                text("Projects").size(24),
                scrollable(project_list.spacing(8)).height(Length::Fill),
                button(text("+ Add Project"))
                    .width(Length::Fill)
                    .on_press(Message::ShowAddProjectForm),
            ]
            .spacing(12),
        )
        .width(Length::Fixed(240.0))
        .height(Length::Fill)
        .into()
    }

    fn view_terminal_area(&self) -> Element<'_, Message> {
        let content = if let Some(selected_id) = self.selected_project {
            if let Some(project) = self
                .config
                .projects
                .iter()
                .find(|project| project.id == selected_id)
            {
                if let Some(terminal) = self.terminals.get(&selected_id) {
                    column![
                        text(format!("Terminal: {}", project.name)).size(16),
                        container(render::terminal_view(
                            selected_id,
                            &terminal.model,
                            move |viewport| Message::TerminalViewportChanged(selected_id, viewport),
                            move |ch| Message::TerminalInput(selected_id, ch),
                        ))
                        .height(Length::Fill),
                    ]
                    .spacing(8)
                } else {
                    column![
                        text(format!("Project: {}", project.name)).size(24),
                        button(text("Open Terminal"))
                            .on_press(Message::OpenTerminal(selected_id)),
                    ]
                    .spacing(8)
                }
            } else {
                column![
                    text("Project not found").size(24),
                    text("Select a project to open a terminal")
                ]
                .spacing(8)
            }
        } else {
            column![
                text("Select a project to open a terminal").size(24),
                text("Terminal area placeholder"),
            ]
            .spacing(8)
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn theme() -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        subscription::pty_output_subscription().map(|(id, bytes)| Message::PtyOutput(id, bytes))
    }
}

fn terminal_size_for_viewport(
    viewport: TerminalViewport,
) -> Option<crate::terminal::model::TerminalSize> {
    if !viewport.width.is_finite() || !viewport.height.is_finite() {
        return None;
    }

    let cols = (viewport.width / render::CELL_WIDTH).floor() as u16;
    let rows = (viewport.height / render::CELL_HEIGHT).floor() as u16;

    if cols == 0 || rows == 0 {
        return None;
    }

    Some(crate::terminal::model::TerminalSize { cols, rows })
}

impl Drop for App {
    fn drop(&mut self) {
        for terminal in self.terminals.values_mut() {
            Self::shutdown_terminal(terminal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{App, Message, TerminalViewport};
    use crate::config::AppConfig;
    use crate::terminal::{
        model::TerminalModel, pty::PtyLifecycle, subscription::subscription_test_lock,
        TerminalState,
    };
    use iced::advanced::subscription::into_recipes;
    use iced::futures::StreamExt;
    use portable_pty::{Child, ChildKiller, ExitStatus};
    use std::collections::HashMap;
    use std::fmt;
    use std::io::{self, Write};
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex, OnceLock};
    use std::time::Duration;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    #[derive(Clone)]
    struct RecordingWriter {
        bytes: Arc<Mutex<Vec<u8>>>,
    }

    #[derive(Clone, Default)]
    struct ChildState {
        killed: usize,
        waited: usize,
        exited: bool,
    }

    #[derive(Clone, Default)]
    struct RecordingChild {
        state: Arc<Mutex<ChildState>>,
    }

    impl fmt::Debug for RecordingChild {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("RecordingChild").finish()
        }
    }

    impl Write for RecordingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.bytes
                .lock()
                .expect("recording writer lock")
                .extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl ChildKiller for RecordingChild {
        fn kill(&mut self) -> io::Result<()> {
            let mut state = self.state.lock().expect("recording child lock");
            state.killed += 1;
            state.exited = true;
            Ok(())
        }

        fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
            Box::new(self.clone())
        }
    }

    impl Child for RecordingChild {
        fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            let state = self.state.lock().expect("recording child lock");
            if state.exited {
                Ok(Some(ExitStatus::with_exit_code(0)))
            } else {
                Ok(None)
            }
        }

        fn wait(&mut self) -> io::Result<ExitStatus> {
            let mut state = self.state.lock().expect("recording child lock");
            state.waited += 1;
            state.exited = true;
            Ok(ExitStatus::with_exit_code(0))
        }

        fn process_id(&self) -> Option<u32> {
            None
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn test_app() -> App {
        let (pty_tx, _pty_rx) = mpsc::channel(super::PTY_CHANNEL_CAPACITY);
        App {
            config: AppConfig::default(),
            selected_project: None,
            add_form: Default::default(),
            terminals: HashMap::new(),
            pty_tx,
        }
    }

    fn insert_test_terminal(
        app: &mut App,
        project_id: Uuid,
        lifecycle: Option<PtyLifecycle>,
    ) -> Arc<Mutex<Vec<u8>>> {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer = RecordingWriter {
            bytes: bytes.clone(),
        };

        app.terminals.insert(
            project_id,
            TerminalState {
                id: project_id,
                project_id: project_id,
                model: TerminalModel::new(120, 40),
                input_buf: String::new(),
                writer: Box::new(writer),
                lifecycle,
                last_size: None,
                resize: Box::new(|_| Ok(())),
            },
        );

        bytes
    }

    fn insert_test_terminal_with_resize(
        app: &mut App,
        project_id: Uuid,
    ) -> Arc<Mutex<Vec<crate::terminal::model::TerminalSize>>> {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let resize_calls = calls.clone();
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer = RecordingWriter {
            bytes: bytes.clone(),
        };

        app.terminals.insert(
            project_id,
            TerminalState {
                id: project_id,
                project_id: project_id,
                model: TerminalModel::new(120, 40),
                input_buf: String::new(),
                writer: Box::new(writer),
                lifecycle: None,
                last_size: None,
                resize: Box::new(move |size| {
                    resize_calls.lock().expect("resize calls lock").push(size);
                    Ok(())
                }),
            },
        );

        calls
    }

    fn with_temp_config_dir<T>(f: impl FnOnce(&PathBuf) -> T) -> T {
        let _guard = env_lock().lock().expect("test env lock");
        let temp_root =
            std::env::temp_dir().join(format!("teminal-panel-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_root).expect("create temp config root");
        let workspace_dir = temp_root.join("workspace");
        std::fs::create_dir_all(&workspace_dir).expect("create temp workspace dir");

        let previous = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", &temp_root);

        let result = f(&workspace_dir);

        match previous {
            Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }

        let _ = std::fs::remove_dir_all(temp_root);
        result
    }

    #[test]
    fn show_and_hide_add_project_form_updates_visibility_and_resets_fields() {
        let mut app = test_app();

        let _ = app.update(Message::ShowAddProjectForm);
        assert!(app.add_form.visible);

        let _ = app.update(Message::FormNameChanged("Local agent".into()));
        assert_eq!(app.add_form.name, "Local agent");

        let _ = app.update(Message::HideAddProjectForm);
        assert!(!app.add_form.visible);
        assert!(app.add_form.name.is_empty());
        assert_eq!(app.add_form.selected_dir, None);
    }

    #[test]
    fn submit_add_form_adds_project_and_resets_form() {
        with_temp_config_dir(|workspace_dir: &PathBuf| {
            let mut app = test_app();

            let _ = app.update(Message::ShowAddProjectForm);
            let _ = app.update(Message::FormNameChanged("Local agent".into()));
            let _ = app.update(Message::ProjectFolderSelected(Some(workspace_dir.clone())));
            let _ = app.update(Message::SubmitAddProjectForm);

            assert_eq!(app.config.projects.len(), 1);
            assert_eq!(app.config.projects[0].name, "Local agent");
            assert_eq!(
                app.config.projects[0].working_dir,
                workspace_dir.clone()
            );
            assert!(!app.add_form.visible);
            assert_eq!(app.add_form.selected_dir, None);

            let persisted = AppConfig::load();
            assert_eq!(persisted.projects.len(), 1);
            assert_eq!(persisted.projects[0].name, "Local agent");
        });
    }

    #[test]
    fn submit_add_form_requires_valid_directory() {
        with_temp_config_dir(|_| {
            let mut app = test_app();

            let _ = app.update(Message::ShowAddProjectForm);
            let _ = app.update(Message::FormNameChanged("Local agent".into()));
            let _ = app.update(Message::ProjectFolderSelected(Some(PathBuf::from("/tmp/missing-directory"))));
            let _ = app.update(Message::SubmitAddProjectForm);

            assert!(app.config.projects.is_empty());
            assert!(app.add_form.visible);
            assert!(AppConfig::load().projects.is_empty());
        });
    }

    #[test]
    fn removing_selected_project_clears_selection() {
        with_temp_config_dir(|workspace_dir: &PathBuf| {
            let mut app = test_app();

            let _ = app.update(Message::AddProject {
                name: "Local agent".into(),
                working_dir: workspace_dir.display().to_string(),
            });

            let project_id = app.config.projects[0].id;
            let _ = app.update(Message::SelectProject(project_id));
            assert_eq!(app.selected_project, Some(project_id));

            let _ = app.update(Message::RemoveProject(project_id));
            assert!(app.config.projects.is_empty());
            assert_eq!(app.selected_project, None);
            assert!(AppConfig::load().projects.is_empty());
        });
    }

    #[test]
    fn input_changed_updates_terminal_input_buffer() {
        let mut app = test_app();
        let project_id = Uuid::new_v4();
        let _ = insert_test_terminal(&mut app, project_id, None);

        let _ = app.update(Message::InputChanged(project_id, "echo hi".into()));

        assert_eq!(
            app.terminals
                .get(&project_id)
                .expect("terminal exists")
                .input_buf,
            "echo hi"
        );
    }

    #[test]
    fn terminal_input_writes_input_and_clears_buffer() {
        let mut app = test_app();
        let project_id = Uuid::new_v4();
        let bytes = insert_test_terminal(&mut app, project_id, None);
        app.terminals
            .get_mut(&project_id)
            .expect("terminal exists")
            .input_buf = "pending".into();

        let _ = app.update(Message::TerminalInput(project_id, "pwd".into()));

        let written = bytes.lock().expect("recording writer lock").clone();
        assert_eq!(written, b"pwd");
        assert!(app
            .terminals
            .get(&project_id)
            .expect("terminal exists")
            .input_buf
            .is_empty());
    }

    #[test]
    fn open_terminal_without_matching_agent_is_noop() {
        let mut app = test_app();
        let _ = app.update(Message::OpenTerminal(Uuid::new_v4()));
        assert!(app.terminals.is_empty());
    }

    #[test]
    fn pty_output_advances_terminal_model_screen() {
        let mut app = test_app();
        let project_id = Uuid::new_v4();
        let _ = insert_test_terminal(&mut app, project_id, None);

        let _ = app.update(Message::PtyOutput(project_id, b"hi".to_vec()));

        let surface = app
            .terminals
            .get(&project_id)
            .expect("terminal exists")
            .model
            .surface();
        assert!(surface.screen_chars_to_string().starts_with("hi"));
    }

    #[test]
    fn ansi_output_updates_surface_instead_of_literal_escape_text() {
        let mut app = test_app();
        let project_id = Uuid::new_v4();
        let _ = insert_test_terminal(&mut app, project_id, None);

        let _ = app.update(Message::PtyOutput(project_id, b"\x1b[31mR".to_vec()));

        let cells = app
            .terminals
            .get(&project_id)
            .expect("terminal exists")
            .model
            .surface()
            .screen_lines();
        assert_eq!(cells[0].visible_cells().next().expect("cell").str(), "R");
        assert!(!app
            .terminals
            .get(&project_id)
            .expect("terminal exists")
            .model
            .surface()
            .screen_chars_to_string()
            .contains("[31m"));
    }

    #[test]
    fn removing_selected_project_shuts_down_terminal_lifecycle() {
        with_temp_config_dir(|workspace_dir: &PathBuf| {
            let mut app = test_app();

            let _ = app.update(Message::AddProject {
                name: "Local agent".into(),
                working_dir: workspace_dir.display().to_string(),
            });

            let project_id = app.config.projects[0].id;
            let child = RecordingChild::default();
            let child_state = child.state.clone();
            let lifecycle = PtyLifecycle::new(Box::new(child));
            let _ = insert_test_terminal(&mut app, project_id, Some(lifecycle));

            let _ = app.update(Message::RemoveProject(project_id));

            let state = child_state.lock().expect("recording child lock");
            assert_eq!(state.killed, 1);
            assert_eq!(state.waited, 1);
        });
    }

    #[test]
    fn dropping_app_shuts_down_terminal_lifecycle() {
        let child = RecordingChild::default();
        let child_state = child.state.clone();

        let mut app = test_app();
        let project_id = Uuid::new_v4();
        let lifecycle = PtyLifecycle::new(Box::new(child));
        let _ = insert_test_terminal(&mut app, project_id, Some(lifecycle));

        drop(app);

        let state = child_state.lock().expect("recording child lock");
        assert_eq!(state.killed, 1);
        assert_eq!(state.waited, 1);
    }

    #[test]
    fn app_new_installs_receiver_for_subscription_stream() {
        let _guard = subscription_test_lock().lock().expect("subscription lock");
        let (app, _) = App::new();
        let terminal_id = Uuid::new_v4();

        app.pty_tx
            .blocking_send((terminal_id, b"echo from app".to_vec()))
            .expect("send pty payload");

        let mut recipes = into_recipes(app.subscription());
        assert_eq!(recipes.len(), 1, "expected one active PTY subscription");

        let mut stream = recipes
            .remove(0)
            .stream(iced::futures::stream::empty().boxed());

        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let message = runtime
            .block_on(async {
                tokio::time::timeout(Duration::from_millis(250), stream.next())
                    .await
                    .expect("subscription stream item within timeout")
            })
            .expect("subscription stream output");

        match message {
            Message::PtyOutput(id, bytes) => {
                assert_eq!(id, terminal_id);
                assert_eq!(bytes, b"echo from app".to_vec());
            }
            other => panic!("unexpected message from subscription: {other:?}"),
        }
    }

    #[test]
    fn terminal_resize_requests_pty_resize_once_per_new_grid_size() {
        let mut app = test_app();
        let project_id = Uuid::new_v4();
        let tracker = insert_test_terminal_with_resize(&mut app, project_id);

        let _ = app.update(Message::TerminalViewportChanged(
            project_id,
            TerminalViewport {
                width: 800.0,
                height: 384.0,
            },
        ));
        let _ = app.update(Message::TerminalViewportChanged(
            project_id,
            TerminalViewport {
                width: 800.0,
                height: 384.0,
            },
        ));

        assert_eq!(
            tracker.lock().expect("tracker lock").as_slice(),
            &[crate::terminal::model::TerminalSize {
                cols: 100,
                rows: 24,
            }]
        );
    }

    #[test]
    fn submit_add_project_form_adds_project_and_resets_form() {
        with_temp_config_dir(|workspace_dir| {
            let mut app = test_app();

            let _ = app.update(Message::ShowAddProjectForm);
            let _ = app.update(Message::FormNameChanged("Local project".into()));
            let _ = app.update(Message::ProjectFolderSelected(Some(workspace_dir.clone())));
            let _ = app.update(Message::SubmitAddProjectForm);

            assert_eq!(app.config.projects.len(), 1);
            assert_eq!(app.config.projects[0].name, "Local project");
            assert_eq!(app.config.projects[0].working_dir, *workspace_dir);
            assert!(!app.add_form.visible);
            assert_eq!(app.add_form.selected_dir, None);
        });
    }

    #[test]
    fn submit_add_project_form_requires_selected_directory() {
        let mut app = test_app();

        let _ = app.update(Message::ShowAddProjectForm);
        let _ = app.update(Message::FormNameChanged("Local project".into()));
        let _ = app.update(Message::SubmitAddProjectForm);

        assert!(app.config.projects.is_empty());
        assert!(app.add_form.visible);
    }

    #[test]
    fn project_folder_selected_none_preserves_existing_selection() {
        let mut app = test_app();

        let _ = app.update(Message::ShowAddProjectForm);
        let _ = app.update(Message::ProjectFolderSelected(Some(std::path::PathBuf::from("/tmp/demo"))));
        let _ = app.update(Message::ProjectFolderSelected(None));

        assert_eq!(
            app.add_form.selected_dir,
            Some(std::path::PathBuf::from("/tmp/demo"))
        );
    }
}
