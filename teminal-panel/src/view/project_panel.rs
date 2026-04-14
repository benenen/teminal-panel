use crate::app::{App, Message};
use crate::project::Connection;
use iced::widget::{button, column, container, mouse_area, row, scrollable, text, text_input};
use iced::{Element, Length, Padding};
use iced_fonts::bootstrap;
use teminal_ui::components::{ContextMenu, TruncatedTooltipText};

impl App {
    pub(crate) fn view_project_panel(&self) -> Element<'_, Message> {
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

            let project_icon = match project.connection {
                Connection::Local => bootstrap::folder().size(14),
                Connection::Ssh { .. } => bootstrap::hdd_network().size(14),
            };

            let project_button = button(
                row![project_icon, text(&project.name).size(13).color(label_color),]
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
                let subtitle = crate::app::project_subtitle(project, &self.config.ssh_services);

                let path_row = container(
                    text(subtitle)
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

                if let Some(remote_files) = self
                    .terminals
                    .get(&project.id)
                    .and_then(|project_terms| project_terms.remote_files.as_ref())
                {
                    if let Some(label) = crate::app::remote_file_status_label(&remote_files.status) {
                        project_col = project_col.push(
                            container(
                                text(label)
                                    .size(10)
                                    .color(iced::Color::from_rgb(0.45, 0.45, 0.45)),
                            )
                            .padding(Padding {
                                top: 2.0,
                                right: 8.0,
                                bottom: 2.0,
                                left: 28.0,
                            }),
                        );
                    }

                    if matches!(remote_files.status, crate::terminal::RemoteFileStatus::Loaded) {
                        for entry in remote_files.entries.iter().take(40) {
                            let icon = if entry.is_dir {
                                bootstrap::folder().size(10)
                            } else {
                                bootstrap::file_earmark().size(10)
                            };

                            project_col = project_col.push(
                                container(
                                    row![icon, text(&entry.name).size(10)]
                                        .spacing(4)
                                        .align_y(iced::alignment::Vertical::Center),
                                )
                                .padding(Padding {
                                    top: 2.0,
                                    right: 8.0,
                                    bottom: 2.0,
                                    left: 28.0,
                                }),
                            );
                        }
                    }
                }

                if let Some(project_terms) = self.terminals.get(&project.id) {
                    for (i, ts) in project_terms.terminals.iter().enumerate() {
                        let is_active_tab = is_selected && i == project_terms.active_index;
                        let is_editing = self.editing_terminal == Some((project.id, i));
                        let display_name = ts.title.as_deref().unwrap_or(&ts.name);

                        let name_field: Element<'_, Message> = if is_editing {
                            text_input("Terminal name", &ts.name)
                                .on_input(move |value| Message::RenameTerminal(project.id, i, value))
                                .on_submit(Message::FinishRenameTerminal)
                                .padding([2, 6])
                                .size(11)
                                .into()
                        } else {
                            button(
                                TruncatedTooltipText::new(display_name)
                                    .max_chars(22)
                                    .size(11)
                                    .width(Length::Fill)
                                    .into_element(),
                            )
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

                let add_term_row = row![bootstrap::plus_lg().size(11), text("New Terminal").size(11),]
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

        let footer_button = row![
            button(bootstrap::gear().size(14))
                .on_press(Message::ToggleSettingsMenu)
                .padding([6, 8])
                .style(button::text)
        ]
        .align_y(iced::alignment::Vertical::Center);

        let footer: Element<'_, Message> = if self.settings_menu_open {
            let menu = ContextMenu::new(
                column![
                    button(text("SSH Service Settings").size(12))
                        .on_press(Message::ShowSshServices)
                        .padding([6, 8])
                        .style(button::text),
                ]
                .spacing(4)
                .into(),
            )
            .into_element();

            iced::widget::stack![
                mouse_area(container(text(""))
                    .width(Length::Fill)
                    .height(Length::Fill))
                    .on_press(Message::HideSettingsMenu),
                container(
                    column![
                        container(menu).padding(Padding {
                            top: 0.0,
                            right: 0.0,
                            bottom: 44.0,
                            left: 0.0,
                        }),
                        footer_button,
                    ]
                    .align_x(iced::alignment::Horizontal::Left)
                )
                .width(Length::Fill)
                .height(Length::Shrink)
            ]
            .into()
        } else {
            footer_button.into()
        };

        container(
            column![
                container(header).padding([8, 10]),
                scrollable(project_list.spacing(2).padding([0, 6])).height(Length::Fill),
                container(footer).padding([8, 10]).height(Length::Fixed(84.0)),
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
}
