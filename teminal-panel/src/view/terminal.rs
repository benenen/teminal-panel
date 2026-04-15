use crate::app::{App, Message};
use crate::terminal::ProjectTerminals;
use iced::widget::{button, column, container, mouse_area, row, scrollable, text};
use iced::{Element, Length};
use iced_fonts::bootstrap;
use teminal_ui::components::TruncatedTooltipText;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanelInteractionMode {
    Direct,
    ClickToActivate,
}

pub(crate) fn panel_interaction_mode(is_active: bool) -> PanelInteractionMode {
    if is_active {
        PanelInteractionMode::Direct
    } else {
        PanelInteractionMode::ClickToActivate
    }
}

impl App {
    pub(crate) fn view_terminal_area(&self) -> Element<'_, Message> {
        let content = if let Some(selected_id) = self.selected_project {
            if let Some(project) = self.config.projects.iter().find(|p| p.id == selected_id) {
                if let Some(project_terms) = self.terminals.get(&selected_id) {
                    if project_terms.terminals.is_empty() {
                        self.view_empty_project(selected_id, &project.name)
                    } else {
                        self.view_terminals(selected_id, &project.name, project_terms)
                    }
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

    pub(crate) fn view_empty_project<'a>(
        &self,
        project_id: Uuid,
        name: &str,
    ) -> Element<'a, Message> {
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

    pub(crate) fn view_terminals<'a>(
        &'a self,
        project_id: Uuid,
        _project_name: &str,
        project_terms: &'a ProjectTerminals,
    ) -> Element<'a, Message> {
        let tab_bar = self.view_tab_bar(project_id, project_terms);

        let terminal_content: Element<'_, Message> = match project_terms.display_mode {
            crate::terminal::DisplayMode::Tabs => {
                if let Some(ts) = project_terms.active_terminal() {
                    iced_term::TerminalView::show(&ts.terminal).map(Message::Terminal)
                } else {
                    text("No terminal").into()
                }
            }
            crate::terminal::DisplayMode::Panel => {
                let panels = project_terms
                    .terminals
                    .iter()
                    .enumerate()
                    .fold(row![], |r, (i, ts)| {
                        let is_active = i == project_terms.active_index;
                        let interaction_mode = panel_interaction_mode(is_active);
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

                        let term_view =
                            iced_term::TerminalView::show(&ts.terminal).map(Message::Terminal);
                        let overlay: Element<'_, Message> = match interaction_mode {
                            PanelInteractionMode::Direct => container(text(""))
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .style(move |_| container::Style::default().background(dim_overlay))
                                .into(),
                            PanelInteractionMode::ClickToActivate => mouse_area(
                                container(text(""))
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .style(move |_| {
                                        container::Style::default().background(dim_overlay)
                                    }),
                            )
                            .on_press(Message::SelectTab(project_id, i))
                            .into(),
                        };
                        let panel = iced::widget::stack![
                            container(term_view)
                                .width(Length::Fill)
                                .height(Length::Fill),
                            overlay,
                        ];

                        r.push(
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

    pub(crate) fn view_tab_bar<'a>(
        &self,
        project_id: Uuid,
        project_terms: &'a ProjectTerminals,
    ) -> Element<'a, Message> {
        let mut tabs = row![].spacing(1);

        for (i, ts) in project_terms.terminals.iter().enumerate() {
            let is_active = i == project_terms.active_index;
            let display_name = ts.title.as_deref().unwrap_or(&ts.name);

            let tab_label = row![
                bootstrap::terminal().size(12),
                button(
                    TruncatedTooltipText::new(display_name)
                        .max_chars(28)
                        .size(12)
                        .into_element(),
                )
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

            tabs = tabs.push(container(tab_label).padding([6, 12]).style(move |_| {
                if is_active {
                    container::Style::default().background(iced::Color::from_rgb(0.18, 0.24, 0.36))
                } else {
                    container::Style::default().background(iced::Color::from_rgb(0.16, 0.16, 0.16))
                }
            }));
        }

        let add_tab = button(bootstrap::plus_lg().size(12))
            .on_press(Message::OpenTerminal(project_id))
            .padding([6, 8])
            .style(button::text);

        let mode_icon = match project_terms.display_mode {
            crate::terminal::DisplayMode::Tabs => bootstrap::layout_split().size(14),
            crate::terminal::DisplayMode::Panel => bootstrap::layout_text_window().size(14),
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
}
