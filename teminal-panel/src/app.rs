use crate::config::AppConfig;
use crate::project::{panel::AddProjectForm, Project};
use crate::terminal::{settings_for_working_dir, DisplayMode, ProjectTerminals, TerminalState};
use iced::widget::{button, column, container, mouse_area, row, scrollable, stack, text, text_input};
use iced::{Element, Length, Padding, Task, Theme};
use iced_fonts::bootstrap;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use teminal_ui::components::{Button, TextInput};
use teminal_ui::containers::Modal;
use uuid::Uuid;

pub struct App {
    pub config: AppConfig,
    pub selected_project: Option<Uuid>,
    pub hovered_project: Option<Uuid>,
    pub expanded_projects: HashSet<Uuid>,
    pub editing_terminal: Option<(Uuid, usize)>,
    pub add_form: AddProjectForm,
    pub terminals: HashMap<Uuid, ProjectTerminals>,
    pub next_terminal_id: u64,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectProject(Uuid),
    AddProject { name: String, working_dir: String },
    RemoveProject(Uuid),
    HoverProject(Option<Uuid>),
    ShowAddProjectForm,
    HideAddProjectForm,
    FormNameChanged(String),
    ChooseProjectFolder,
    ProjectFolderSelected(Option<PathBuf>),
    SubmitAddProjectForm,
    OpenTerminal(Uuid),
    ToggleProjectExpanded(Uuid),
    SelectTab(Uuid, usize),
    CloseTab(Uuid, usize),
    StartRenameTerminal(Uuid, usize),
    RenameTerminal(Uuid, usize, String),
    FinishRenameTerminal,
    ToggleDisplayMode(Uuid),
    Terminal(iced_term::Event),
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                config: AppConfig::load(),
                selected_project: None,
                hovered_project: None,
                expanded_projects: HashSet::new(),
                editing_terminal: None,
                add_form: Default::default(),
                terminals: HashMap::new(),
                next_terminal_id: 1,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectProject(id) => {
                self.selected_project = Some(id);
                self.expanded_projects.insert(id);
            }
            Message::AddProject { name, working_dir } => {
                self.add_local_project(name, PathBuf::from(working_dir));
            }
            Message::RemoveProject(id) => {
                self.config.projects.retain(|project| project.id != id);
                self.terminals.remove(&id);

                if self.selected_project == Some(id) {
                    self.selected_project = None;
                }
                if self.hovered_project == Some(id) {
                    self.hovered_project = None;
                }

                self.config.save();
            }
            Message::HoverProject(id) => {
                self.hovered_project = id;
            }
            Message::ShowAddProjectForm => {
                self.add_form.visible = true;
            }
            Message::HideAddProjectForm => {
                self.add_form = Default::default();
            }
            Message::FormNameChanged(value) => {
                self.add_form.name = value;
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
                if let Some(project) = self.config.projects.iter().find(|p| p.id == project_id) {
                    match iced_term::Terminal::new(
                        self.next_terminal_id,
                        settings_for_working_dir(&project.working_dir),
                    ) {
                        Ok(terminal) => {
                            self.next_terminal_id += 1;
                            let widget_id = terminal.widget_id().clone();

                            let project_name = project.name.clone();
                            let project_terms = self
                                .terminals
                                .entry(project_id)
                                .or_insert_with(ProjectTerminals::new);
                            let term_num = project_terms.terminals.len() + 1;

                            project_terms.terminals.push(TerminalState {
                                terminal,
                                name: format!("{} * {}", project_name, term_num),
                                title: None,
                            });
                            project_terms.active_index = project_terms.terminals.len() - 1;

                            self.expanded_projects.insert(project_id);

                            return iced_term::TerminalView::focus(widget_id);
                        }
                        Err(e) => {
                            eprintln!("Failed to create terminal: {e}");
                        }
                    }
                }
            }
            Message::ToggleProjectExpanded(id) => {
                if !self.expanded_projects.remove(&id) {
                    self.expanded_projects.insert(id);
                }
            }
            Message::SelectTab(project_id, index) => {
                self.selected_project = Some(project_id);
                self.expanded_projects.insert(project_id);
                if let Some(project_terms) = self.terminals.get_mut(&project_id) {
                    if index < project_terms.terminals.len() {
                        project_terms.active_index = index;
                        let widget_id = project_terms.terminals[index].terminal.widget_id().clone();
                        return iced_term::TerminalView::focus(widget_id);
                    }
                }
            }
            Message::CloseTab(project_id, index) => {
                if let Some(project_terms) = self.terminals.get_mut(&project_id) {
                    project_terms.remove_terminal(index);
                    if project_terms.terminals.is_empty() {
                        self.terminals.remove(&project_id);
                    }
                }
                if self.editing_terminal == Some((project_id, index)) {
                    self.editing_terminal = None;
                }
            }
            Message::StartRenameTerminal(project_id, index) => {
                self.editing_terminal = Some((project_id, index));
            }
            Message::RenameTerminal(project_id, index, new_name) => {
                if let Some(project_terms) = self.terminals.get_mut(&project_id) {
                    if let Some(ts) = project_terms.terminals.get_mut(index) {
                        ts.name = new_name;
                    }
                }
            }
            Message::FinishRenameTerminal => {
                self.editing_terminal = None;
            }
            Message::ToggleDisplayMode(project_id) => {
                if let Some(project_terms) = self.terminals.get_mut(&project_id) {
                    project_terms.display_mode = match project_terms.display_mode {
                        DisplayMode::Tabs => DisplayMode::Panel,
                        DisplayMode::Panel => DisplayMode::Tabs,
                    };
                }
            }
            Message::Terminal(iced_term::Event::BackendCall(term_id, cmd)) => {
                let mut closed = None;

                for (project_id, project_terms) in self.terminals.iter_mut() {
                    if let Some((idx, ts)) = project_terms
                        .terminals
                        .iter_mut()
                        .enumerate()
                        .find(|(_, ts)| ts.terminal.id == term_id)
                    {
                        self.selected_project = Some(*project_id);
                        self.expanded_projects.insert(*project_id);
                        project_terms.active_index = idx;

                        match ts.terminal.handle(iced_term::Command::ProxyToBackend(cmd)) {
                            iced_term::actions::Action::Shutdown => {
                                closed = Some((*project_id, idx));
                            }
                            iced_term::actions::Action::ChangeTitle(title) => {
                                ts.title = Some(title);
                            }
                            iced_term::actions::Action::Ignore => {}
                        }
                        break;
                    }
                }

                if let Some((project_id, idx)) = closed {
                    if let Some(project_terms) = self.terminals.get_mut(&project_id) {
                        project_terms.remove_terminal(idx);
                        if project_terms.terminals.is_empty() {
                            self.terminals.remove(&project_id);
                        }
                    }
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

        self.config
            .projects
            .push(Project::new_local(name, working_dir));
        self.config.save();
        true
    }

    pub fn view(&self) -> Element<'_, Message> {
        let main_content = row![self.view_project_panel(), self.view_terminal_area()]
            .spacing(0)
            .padding(0);

        if self.add_form.visible {
            let selected_dir = self
                .add_form
                .selected_dir
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "No folder selected".into());

            let form_content = column![
                TextInput::new("Project Name", &self.add_form.name)
                    .on_input(Message::FormNameChanged)
                    .on_submit(Message::SubmitAddProjectForm)
                    .into_element(),
                row![
                    text(selected_dir).size(12),
                    button(bootstrap::folder_plus().size(14))
                        .on_press(Message::ChooseProjectFolder)
                        .padding([4, 8])
                        .style(button::secondary),
                ]
                .spacing(8)
                .align_y(iced::alignment::Vertical::Center),
                row![
                    Button::new("Add")
                        .width(Length::Fill)
                        .on_press(Message::SubmitAddProjectForm)
                        .into_element(),
                    Button::new("Cancel")
                        .width(Length::Fill)
                        .on_press(Message::HideAddProjectForm)
                        .into_element(),
                ]
                .spacing(8),
            ]
            .spacing(16);

            let modal = Modal::new(form_content.into())
                .with_title("Add Project")
                .into_element();

            let overlay = container(modal)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| {
                    container::Style::default()
                        .background(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5))
                })
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            stack![main_content, overlay,]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            main_content.into()
        }
    }

    fn view_project_panel(&self) -> Element<'_, Message> {
        let project_list = self.config.projects.iter().fold(column![], |col, project| {
            let is_selected = self.selected_project == Some(project.id);
            let is_expanded = self.expanded_projects.contains(&project.id);

            let chevron = if is_expanded {
                bootstrap::chevron_down().size(10)
            } else {
                bootstrap::chevron_right().size(10)
            };

            let show_close = is_selected || self.hovered_project == Some(project.id);
            let row_bg = if is_selected {
                Some(iced::Color::from_rgb(0.18, 0.24, 0.36))
            } else {
                None
            };
            let row_border = if is_selected {
                iced::Color::from_rgb(0.3, 0.5, 0.9)
            } else {
                iced::Color::TRANSPARENT
            };
            let label_color = if is_selected {
                iced::Color::WHITE
            } else {
                iced::Color::from_rgb(0.85, 0.85, 0.85)
            };

            let project_button = button(
                row![bootstrap::folder().size(14), text(&project.name).size(13).color(label_color),]
                    .spacing(6)
                    .align_y(iced::alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .style(button::text)
            .padding([6, 8])
            .on_press(Message::SelectProject(project.id));

            let chevron_btn = button(chevron)
                .on_press(Message::ToggleProjectExpanded(project.id))
                .padding([4, 4])
                .style(button::text);

            let close_btn: Element<'_, Message> = if show_close {
                button(bootstrap::x_lg().size(10))
                    .on_press(Message::RemoveProject(project.id))
                    .padding([4, 4])
                    .style(button::text)
                    .into()
            } else {
                container(text(" ").size(10)).padding([4, 4]).into()
            };

            let row_content = row![chevron_btn, project_button, close_btn,]
                .spacing(2)
                .align_y(iced::alignment::Vertical::Center);

            let row_container = container(row_content)
                .width(Length::Fill)
                .style(move |_| {
                    let mut style = container::Style::default().border(iced::Border {
                        color: row_border,
                        width: if row_bg.is_some() { 1.0 } else { 0.0 },
                        radius: 6.into(),
                    });
                    if let Some(bg) = row_bg {
                        style = style.background(bg);
                    }
                    style
                })
                .padding([0, 2]);

            let name_row = mouse_area(row_container)
                .on_enter(Message::HoverProject(Some(project.id)))
                .on_exit(Message::HoverProject(None));

            let mut project_col = column![name_row].spacing(0);

            if is_expanded {
                let path_row = container(
                    text(project.working_dir.display().to_string())
                        .size(10)
                        .color(iced::Color::from_rgb(0.45, 0.45, 0.45)),
                )
                .padding(Padding {
                    top: 2.0,
                    right: 8.0,
                    bottom: 2.0,
                    left: 28.0,
                });

                project_col = project_col.push(path_row);

                if let Some(project_terms) = self.terminals.get(&project.id) {
                    for (i, ts) in project_terms.terminals.iter().enumerate() {
                        let is_active_tab = is_selected && i == project_terms.active_index;
                        let is_editing = self.editing_terminal == Some((project.id, i));

                        let name_field: Element<'_, Message> = if is_editing {
                            text_input("Terminal name", &ts.name)
                                .on_input(move |value| Message::RenameTerminal(project.id, i, value))
                                .on_submit(Message::FinishRenameTerminal)
                                .padding([2, 6])
                                .size(11)
                                .into()
                        } else {
                            button(text(&ts.name).size(11).width(Length::Fill))
                                .on_press(Message::SelectTab(project.id, i))
                                .style(button::text)
                                .padding(0)
                                .width(Length::Fill)
                                .into()
                        };

                        let term_row = row![
                            bootstrap::terminal().size(11),
                            name_field,
                            button(bootstrap::pencil().size(9))
                                .on_press(Message::StartRenameTerminal(project.id, i))
                                .padding([1, 3])
                                .style(button::text),
                            button(bootstrap::x_lg().size(9))
                                .on_press(Message::CloseTab(project.id, i))
                                .padding([1, 3])
                                .style(button::text),
                        ]
                        .spacing(6)
                        .align_y(iced::alignment::Vertical::Center);

                        project_col = project_col.push(
                            container(term_row)
                                .width(Length::Fill)
                                .padding(Padding {
                                    top: 4.0,
                                    right: 8.0,
                                    bottom: 4.0,
                                    left: 28.0,
                                })
                                .style(move |_| {
                                    if is_active_tab {
                                        container::Style::default()
                                            .background(iced::Color::from_rgb(0.18, 0.24, 0.36))
                                    } else {
                                        container::Style::default()
                                    }
                                }),
                        );
                    }
                }

                // Add terminal button inside expanded project
                let add_term_row =
                    row![bootstrap::plus_lg().size(11), text("New Terminal").size(11),]
                        .spacing(4)
                        .align_y(iced::alignment::Vertical::Center);

                project_col = project_col.push(
                    button(add_term_row)
                        .width(Length::Fill)
                        .padding(Padding {
                            top: 4.0,
                            right: 8.0,
                            bottom: 4.0,
                            left: 28.0,
                        })
                        .style(button::text)
                        .on_press(Message::OpenTerminal(project.id)),
                );
            }

            col.push(project_col)
        });

        let header = row![
            text("Projects")
                .size(13)
                .color(iced::Color::from_rgb(0.6, 0.6, 0.6))
                .width(Length::Fill),
            button(bootstrap::plus_lg().size(14))
                .on_press(Message::ShowAddProjectForm)
                .padding([4, 6])
                .style(button::text),
        ]
        .align_y(iced::alignment::Vertical::Center);

        container(
            column![
                container(header).padding([8, 10]),
                scrollable(project_list.spacing(2).padding([0, 6])).height(Length::Fill),
            ]
            .spacing(0),
        )
        .width(Length::Fixed(220.0))
        .height(Length::Fill)
        .style(|_| {
            container::Style::default()
                .background(iced::Color::from_rgb(0.1, 0.1, 0.1))
                .border(iced::Border {
                    color: iced::Color::from_rgb(0.18, 0.18, 0.18),
                    width: 1.0,
                    radius: 0.into(),
                })
        })
        .into()
    }

    fn view_terminal_area(&self) -> Element<'_, Message> {
        let content = if let Some(selected_id) = self.selected_project {
            if let Some(project) = self.config.projects.iter().find(|p| p.id == selected_id) {
                if let Some(project_terms) = self.terminals.get(&selected_id) {
                    self.view_terminals(selected_id, &project.name, project_terms)
                } else {
                    self.view_empty_project(selected_id, &project.name)
                }
            } else {
                column![text("Project not found").size(14)].into()
            }
        } else {
            container(
                column![
                    bootstrap::terminal_fill()
                        .size(48)
                        .color(iced::Color::from_rgb(0.25, 0.25, 0.25)),
                    text("Select a project")
                        .size(14)
                        .color(iced::Color::from_rgb(0.4, 0.4, 0.4)),
                ]
                .spacing(12)
                .align_x(iced::alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| {
                container::Style::default().background(iced::Color::from_rgb(0.08, 0.08, 0.08))
            })
            .into()
    }

    fn view_empty_project<'a>(&self, project_id: Uuid, name: &str) -> Element<'a, Message> {
        container(
            column![
                bootstrap::terminal_plus()
                    .size(48)
                    .color(iced::Color::from_rgb(0.3, 0.3, 0.3)),
                text(name.to_string()).size(16),
                button(
                    row![
                        bootstrap::terminal_plus().size(14),
                        text("Open Terminal").size(13),
                    ]
                    .spacing(6)
                    .align_y(iced::alignment::Vertical::Center),
                )
                .on_press(Message::OpenTerminal(project_id))
                .padding([8, 16])
                .style(button::primary),
            ]
            .spacing(12)
            .align_x(iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    fn view_terminals<'a>(
        &'a self,
        project_id: Uuid,
        _project_name: &str,
        project_terms: &'a ProjectTerminals,
    ) -> Element<'a, Message> {
        let tab_bar = self.view_tab_bar(project_id, project_terms);

        let terminal_content: Element<'_, Message> = match project_terms.display_mode {
            DisplayMode::Tabs => {
                if let Some(ts) = project_terms.active_terminal() {
                    iced_term::TerminalView::show(&ts.terminal).map(Message::Terminal)
                } else {
                    text("No terminal").into()
                }
            }
            DisplayMode::Panel => {
                let panels = project_terms
                    .terminals
                    .iter()
                    .enumerate()
                    .fold(row![], |r, (i, ts)| {
                        let is_active = i == project_terms.active_index;
                        let border_color = if is_active {
                            iced::Color::from_rgb(0.3, 0.5, 0.9)
                        } else {
                            iced::Color::from_rgb(0.18, 0.18, 0.18)
                        };
                        let dim_overlay = if is_active {
                            iced::Color::from_rgba(0.0, 0.0, 0.0, 0.0)
                        } else {
                            iced::Color::from_rgba(0.0, 0.0, 0.0, 0.38)
                        };

                        let term_view = iced_term::TerminalView::show(&ts.terminal).map(Message::Terminal);
                        let panel = stack![
                            container(term_view)
                                .width(Length::Fill)
                                .height(Length::Fill),
                            container(text(""))
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .style(move |_| container::Style::default().background(dim_overlay)),
                        ];

                        r.push(
                            mouse_area(
                                container(panel)
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .style(move |_| {
                                        container::Style::default()
                                            .background(iced::Color::from_rgb(0.08, 0.08, 0.08))
                                            .border(iced::Border {
                                                color: border_color,
                                                width: if is_active { 2.0 } else { 1.0 },
                                                radius: 0.into(),
                                            })
                                    })
                                    .padding(0),
                            )
                            .on_press(Message::SelectTab(project_id, i))
                        )
                    })
                    .spacing(6);
                row!(panels).into()
            }
        };

        column![
            tab_bar,
            container(terminal_content)
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(0)
        .into()
    }

    fn view_tab_bar<'a>(
        &self,
        project_id: Uuid,
        project_terms: &'a ProjectTerminals,
    ) -> Element<'a, Message> {
        let mut tabs = row![].spacing(1);

        for (i, ts) in project_terms.terminals.iter().enumerate() {
            let is_active = i == project_terms.active_index;

            let tab_label = row![
                bootstrap::terminal().size(12),
                button(text(&ts.name).size(12))
                    .on_press(Message::SelectTab(project_id, i))
                    .style(button::text)
                    .padding(0),
                button(bootstrap::x_lg().size(10))
                    .on_press(Message::CloseTab(project_id, i))
                    .padding([2, 4])
                    .style(button::text),
            ]
            .spacing(6)
            .align_y(iced::alignment::Vertical::Center);

            tabs = tabs.push(
                container(tab_label)
                    .padding([6, 12])
                    .style(move |_| {
                        if is_active {
                            container::Style::default()
                                .background(iced::Color::from_rgb(0.18, 0.24, 0.36))
                        } else {
                            container::Style::default()
                                .background(iced::Color::from_rgb(0.16, 0.16, 0.16))
                        }
                    }),
            );
        }

        let add_tab = button(bootstrap::plus_lg().size(12))
            .on_press(Message::OpenTerminal(project_id))
            .padding([6, 8])
            .style(button::text);

        let mode_icon = match project_terms.display_mode {
            DisplayMode::Tabs => bootstrap::layout_split().size(14),
            DisplayMode::Panel => bootstrap::layout_text_window().size(14),
        };
        let mode_btn = button(mode_icon)
            .on_press(Message::ToggleDisplayMode(project_id))
            .padding([6, 8])
            .style(button::text);

        container(
            row![
                scrollable(tabs).direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::default(),
                )),
                add_tab,
                mode_btn,
            ]
            .spacing(4)
            .align_y(iced::alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .padding([2, 4])
        .style(|_| {
            container::Style::default()
                .background(iced::Color::from_rgb(0.12, 0.12, 0.12))
                .border(iced::Border {
                    color: iced::Color::from_rgb(0.18, 0.18, 0.18),
                    width: 1.0,
                    radius: 0.into(),
                })
        })
        .into()
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch(
            self.terminals
                .values()
                .flat_map(|pt| pt.terminals.iter())
                .map(|ts| ts.terminal.subscription().map(Message::Terminal)),
        )
    }
}

#[cfg(test)]
#[path = "app_test.rs"]
mod tests;
