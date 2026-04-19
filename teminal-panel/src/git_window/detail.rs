use super::{theme, Message, SelectedFileDetail};
use iced::widget::{button, column, container, row, scrollable, text, text_editor};
use iced::{Alignment, Element, Font, Length};

pub(super) fn view_selected_detail(detail: &SelectedFileDetail) -> Element<'_, Message> {
    let header = column![
        text(detail.path.display().to_string())
            .size(14)
            .color(theme::TEXT_PRIMARY),
        row![
            text(status_label(detail))
                .size(11)
                .color(status_color(detail)),
            text(if detail.staged {
                "Staged"
            } else {
                "Working Tree"
            })
            .size(11)
            .color(theme::TEXT_TERTIARY),
            text(if detail.dirty {
                "Unsaved edits"
            } else {
                "Saved"
            })
            .size(11)
            .color(theme::TEXT_TERTIARY),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
    ]
    .spacing(6);

    let can_render_text_detail = detail.base_text.is_some() || detail.draft.is_some();
    let body = if detail.content_kind == super::git_data::FileContentKind::Binary {
        view_binary_detail(detail)
    } else if can_render_text_detail {
        view_text_detail(detail)
    } else if let Some(error) = &detail.detail_error {
        view_error_detail(error)
    } else {
        view_error_detail("File detail is unavailable.")
    };

    let mut content = column![header].spacing(16).padding(20);

    if let Some(error) = &detail.detail_error {
        content = content.push(view_error_banner(error));
    }

    content = content.push(body);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn view_text_detail(detail: &SelectedFileDetail) -> Element<'_, Message> {
    let action_row: Element<'_, Message> = if detail.staged {
        text("Staged snapshots are read-only in this phase.")
            .size(12)
            .color(theme::TEXT_TERTIARY)
            .into()
    } else {
        row![
            button(text("Apply").size(12))
                .on_press(Message::ApplySelectedFile)
                .padding([8, 14]),
            button(text("Discard").size(12))
                .on_press(Message::DiscardSelectedFile)
                .padding([8, 14]),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    };

    let compare_row = row![
        container(
            column![
                text("Base").size(12).color(theme::TEXT_TERTIARY),
                scrollable(
                    text(detail.base_text.as_deref().unwrap_or(""))
                        .size(12)
                        .font(Font::MONOSPACE)
                        .color(theme::TEXT_SECONDARY)
                )
            ]
            .spacing(8)
        )
        .width(Length::Fill)
        .height(Length::Fill),
        container(
            column![
                text(if detail.staged {
                    "Staged"
                } else {
                    "Working Tree"
                })
                .size(12)
                .color(theme::TEXT_TERTIARY),
                view_worktree_editor(detail)
            ]
            .spacing(8)
        )
        .width(Length::Fill)
        .height(Length::Fill),
    ]
    .spacing(16)
    .height(Length::Fill);

    column![compare_row, action_row]
        .spacing(16)
        .height(Length::Fill)
        .into()
}

fn view_worktree_editor(detail: &SelectedFileDetail) -> Element<'_, Message> {
    if let Some(draft) = detail.draft.as_ref() {
        if detail.staged {
            return scrollable(
                text(draft.text())
                    .size(12)
                    .font(Font::MONOSPACE)
                    .color(theme::TEXT_PRIMARY),
            )
            .into();
        }

        text_editor(draft)
            .on_action(Message::EditSelectedFile)
            .font(Font::MONOSPACE)
            .size(12)
            .height(Length::Fill)
            .into()
    } else {
        container(
            text("Working tree content is unavailable.")
                .size(12)
                .color(theme::GIT_DELETED),
        )
        .height(Length::Fill)
        .into()
    }
}

fn view_binary_detail(detail: &SelectedFileDetail) -> Element<'_, Message> {
    let diff_summary = detail
        .diff
        .as_deref()
        .filter(|diff| !diff.is_empty())
        .unwrap_or("Binary file selected");

    column![
        text("Binary file").size(13).color(theme::TEXT_PRIMARY),
        text("Editing is not supported for binary files.")
            .size(12)
            .color(theme::TEXT_SECONDARY),
        scrollable(
            text(diff_summary)
                .size(12)
                .font(Font::MONOSPACE)
                .color(theme::TEXT_TERTIARY)
        )
    ]
    .spacing(12)
    .into()
}

fn view_error_detail(error: &str) -> Element<'_, Message> {
    text(error.to_string())
        .size(13)
        .color(theme::GIT_DELETED)
        .into()
}

fn view_error_banner(error: &str) -> Element<'_, Message> {
    container(text(error).size(12).color(theme::GIT_DELETED))
        .padding(10)
        .width(Length::Fill)
        .into()
}

fn status_label(detail: &SelectedFileDetail) -> &'static str {
    match detail.status {
        super::git_data::FileStatus::Added => "Added",
        super::git_data::FileStatus::Modified => "Modified",
        super::git_data::FileStatus::Deleted => "Deleted",
    }
}

fn status_color(detail: &SelectedFileDetail) -> iced::Color {
    match detail.status {
        super::git_data::FileStatus::Added => theme::GIT_ADDED,
        super::git_data::FileStatus::Modified => theme::GIT_MODIFIED,
        super::git_data::FileStatus::Deleted => theme::GIT_DELETED,
    }
}
