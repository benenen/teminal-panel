use crate::app::{App, Message, SshAuthType};
use crate::project::{panel::ProjectConnectionKind, SshAuth};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length, Padding};
use iced_fonts::bootstrap;
use teminal_ui::components::{Button, TextInput};
use teminal_ui::containers::Modal;

impl App {
    pub(crate) fn view_add_project_overlay(&self) -> Element<'_, Message> {
        let selected_dir = self
            .add_form
            .selected_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| {
                if self.add_form.connection_kind == ProjectConnectionKind::Ssh {
                    "No remote path set".into()
                } else {
                    "No folder selected".into()
                }
            });

        let connection_row = row![
            button(text("Local").size(12))
                .on_press(Message::FormConnectionKindChanged(ProjectConnectionKind::Local))
                .padding([4, 8])
                .style(if self.add_form.connection_kind == ProjectConnectionKind::Local {
                    button::primary
                } else {
                    button::secondary
                }),
            button(text("SSH").size(12))
                .on_press(Message::FormConnectionKindChanged(ProjectConnectionKind::Ssh))
                .padding([4, 8])
                .style(if self.add_form.connection_kind == ProjectConnectionKind::Ssh {
                    button::primary
                } else {
                    button::secondary
                }),
        ]
        .spacing(8);

        let mut form_content = column![
            TextInput::new("Project Name", &self.add_form.name)
                .on_input(Message::FormNameChanged)
                .on_submit(Message::SubmitAddProjectForm)
                .into_element(),
            connection_row,
            {
                let path_input: Element<'_, Message> = if self.add_form.connection_kind
                    == ProjectConnectionKind::Ssh
                {
                    column![
                        text("Remote Working Directory").size(12),
                        TextInput::new("/srv/app", &selected_dir)
                            .on_input(|value| {
                                Message::ProjectFolderSelected(Some(std::path::PathBuf::from(value)))
                            })
                            .into_element(),
                    ]
                    .spacing(6)
                    .into()
                } else {
                    row![
                        text(selected_dir).size(12),
                        button(bootstrap::folder_plus().size(14))
                            .on_press(Message::ChooseProjectFolder)
                            .padding([4, 8])
                            .style(button::secondary),
                    ]
                    .spacing(8)
                    .align_y(iced::alignment::Vertical::Center)
                    .into()
                };

                path_input
            },
        ]
        .spacing(16);

        if self.add_form.connection_kind == ProjectConnectionKind::Ssh {
            let selected_label = self
                .add_form
                .ssh_service_id
                .and_then(|id| self.config.ssh_services.iter().find(|service| service.id == id))
                .map(|service| format!("Selected: {}", service.name))
                .unwrap_or_else(|| "Select an SSH service".into());

            let ssh_services = self
                .config
                .ssh_services
                .iter()
                .fold(column![text(selected_label).size(12)], |col, service| {
                    col.push(
                        button(text(&service.name).size(12))
                            .on_press(Message::FormSshServiceSelected(service.id))
                            .padding([4, 8])
                            .style(if self.add_form.ssh_service_id == Some(service.id) {
                                button::primary
                            } else {
                                button::secondary
                            }),
                    )
                })
                .spacing(8);

            form_content = form_content.push(ssh_services);
        }

        form_content = form_content.push(
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
        );

        let modal = Modal::new(form_content.into())
            .with_title("Add Project")
            .into_element();

        container(modal)
            .width(Length::Fill)
            .height(Length::Shrink)
            .style(|_| {
                container::Style::default().background(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5))
            })
            .center_x(Length::Fill)
            .into()
    }

    pub(crate) fn view_settings_menu_overlay(&self) -> Element<'_, Message> {
        let menu = container(
            column![
                button(text("SSH Service Settings").size(12))
                    .on_press(Message::ShowSshServices)
                    .padding([6, 8])
                    .style(button::text),
            ]
            .spacing(4),
        )
        .padding(8)
        .style(|_| {
            container::Style::default()
                .background(iced::Color::from_rgb(0.14, 0.14, 0.14))
                .border(iced::Border {
                    color: iced::Color::from_rgb(0.22, 0.22, 0.22),
                    width: 1.0,
                    radius: 8.into(),
                })
        });

        container(row![container(menu).width(Length::Fixed(220.0)), container(text(""))])
            .width(Length::Fill)
            .height(Length::Shrink)
            .padding(Padding {
                top: 0.0,
                right: 0.0,
                bottom: 12.0,
                left: 12.0,
            })
            .into()
    }

    pub(crate) fn view_ssh_services_overlay(&self) -> Element<'_, Message> {
        let services = self
            .config
            .ssh_services
            .iter()
            .fold(column![].spacing(8), |col, service| {
                let auth_text = match service.auth {
                    SshAuth::Password(_) => "password",
                    SshAuth::Key { .. } => "key",
                    SshAuth::Agent => "agent",
                };

                col.push(
                    container(
                        row![
                            column![
                                text(&service.name).size(13),
                                text(format!(
                                    "{}@{}:{} · {}",
                                    service.user, service.host, service.port, auth_text
                                ))
                                .size(11)
                                .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
                            ]
                            .spacing(2)
                            .width(Length::Fill),
                            button(bootstrap::pencil().size(10))
                                .on_press(Message::EditSshService(service.id))
                                .padding([4, 6])
                                .style(button::text),
                            button(bootstrap::trash().size(10))
                                .on_press(Message::DeleteSshService(service.id))
                                .padding([4, 6])
                                .style(button::text),
                        ]
                        .align_y(iced::alignment::Vertical::Center),
                    )
                    .padding([8, 10])
                    .style(|_| {
                        container::Style::default().background(iced::Color::from_rgb(0.18, 0.18, 0.18))
                    }),
                )
            });

        let auth_row = row![
            button(text("Agent").size(12))
                .on_press(Message::SshServiceAuthTypeChanged(SshAuthType::Agent))
                .padding([4, 8])
                .style(if self.ssh_service_form.auth_type == SshAuthType::Agent {
                    button::primary
                } else {
                    button::secondary
                }),
            button(text("Password").size(12))
                .on_press(Message::SshServiceAuthTypeChanged(SshAuthType::Password))
                .padding([4, 8])
                .style(if self.ssh_service_form.auth_type == SshAuthType::Password {
                    button::primary
                } else {
                    button::secondary
                }),
            button(text("Key").size(12))
                .on_press(Message::SshServiceAuthTypeChanged(SshAuthType::Key))
                .padding([4, 8])
                .style(if self.ssh_service_form.auth_type == SshAuthType::Key {
                    button::primary
                } else {
                    button::secondary
                }),
        ]
        .spacing(8);

        let mut form = column![
            text(if self.editing_ssh_service.is_some() {
                "Edit SSH Service"
            } else {
                "Add SSH Service"
            })
            .size(14),
            text_input("Name", &self.ssh_service_form.name)
                .on_input(Message::SshServiceNameChanged)
                .padding([6, 8]),
            text_input("Host", &self.ssh_service_form.host)
                .on_input(Message::SshServiceHostChanged)
                .padding([6, 8]),
            row![
                text_input("Port", &self.ssh_service_form.port)
                    .on_input(Message::SshServicePortChanged)
                    .padding([6, 8]),
                text_input("User", &self.ssh_service_form.user)
                    .on_input(Message::SshServiceUserChanged)
                    .padding([6, 8]),
            ]
            .spacing(8),
            auth_row,
        ]
        .spacing(10);

        match self.ssh_service_form.auth_type {
            SshAuthType::Password => {
                form = form.push(
                    text_input("Password", &self.ssh_service_form.password)
                        .on_input(Message::SshServicePasswordChanged)
                        .padding([6, 8]),
                );
            }
            SshAuthType::Key => {
                form = form
                    .push(
                        text_input("Key Path", &self.ssh_service_form.key_path)
                            .on_input(Message::SshServiceKeyPathChanged)
                            .padding([6, 8]),
                    )
                    .push(
                        text_input("Passphrase", &self.ssh_service_form.key_passphrase)
                            .on_input(Message::SshServiceKeyPassphraseChanged)
                            .padding([6, 8]),
                    );
            }
            SshAuthType::Agent => {}
        }

        form = form.push(
            row![
                Button::new(if self.editing_ssh_service.is_some() { "Save" } else { "Add" })
                    .width(Length::Fill)
                    .on_press(Message::SubmitSshServiceForm)
                    .into_element(),
                Button::new("Reset")
                    .width(Length::Fill)
                    .on_press(Message::CancelSshServiceForm)
                    .into_element(),
            ]
            .spacing(8),
        );

        let content = column![
            row![
                text("Saved Services").size(14).width(Length::Fill),
                button(bootstrap::plus_lg().size(12))
                    .on_press(Message::ShowAddSshServiceForm)
                    .padding([4, 6])
                    .style(button::text),
                button(bootstrap::x_lg().size(12))
                    .on_press(Message::HideOverlay)
                    .padding([4, 6])
                    .style(button::text),
            ]
            .align_y(iced::alignment::Vertical::Center),
            scrollable(services).height(Length::Fixed(180.0)),
            form,
        ]
        .spacing(16);

        let modal = Modal::new(content.into())
            .with_title("SSH Service Settings")
            .width(Length::Fixed(720.0))
            .into_element();

        container(modal)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| {
                container::Style::default().background(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5))
            })
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
