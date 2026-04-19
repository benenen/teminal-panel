use super::git_data::CommitNode;
use super::theme;
use super::Message;
use iced::alignment;
use iced::mouse;
use iced::widget::canvas;
use iced::widget::{container, scrollable};
use iced::{Element, Font, Length, Pixels, Point, Rectangle, Renderer, Theme};

const ROW_HEIGHT: f32 = 52.0;
const GRAPH_X: f32 = 28.0;
const CONTENT_PADDING: f32 = 20.0;

pub fn view_commit_graph(commits: &[CommitNode]) -> Element<'_, Message> {
    let canvas_height = (commits.len().max(1) as f32 * ROW_HEIGHT) + CONTENT_PADDING * 2.0;
    let canvas = canvas::Canvas::new(CommitGraph { commits })
        .width(Length::Fill)
        .height(Length::Fixed(canvas_height));

    container(scrollable(canvas).width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

struct CommitGraph<'a> {
    commits: &'a [CommitNode],
}

impl<MessageType> canvas::Program<MessageType> for CommitGraph<'_> {
    type State = canvas::Cache;

    fn draw(
        &self,
        cache: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        vec![cache.draw(renderer, bounds.size(), |frame| {
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), theme::BG_SECONDARY);

            if self.commits.is_empty() {
                frame.fill_text(canvas::Text {
                    content: "No commit history available".into(),
                    position: Point::new(CONTENT_PADDING, CONTENT_PADDING),
                    color: theme::TEXT_SECONDARY,
                    size: Pixels::from(14.0),
                    line_height: iced::widget::text::LineHeight::default(),
                    font: Font::DEFAULT,
                    align_x: iced::alignment::Horizontal::Left.into(),
                    align_y: alignment::Vertical::Top,
                    shaping: iced::widget::text::Shaping::Basic,
                    max_width: f32::INFINITY,
                });
                return;
            }

            for (index, commit) in self.commits.iter().enumerate() {
                let center_y = CONTENT_PADDING + index as f32 * ROW_HEIGHT + 12.0;

                if index + 1 < self.commits.len() {
                    let connector = canvas::Path::line(
                        Point::new(GRAPH_X, center_y),
                        Point::new(GRAPH_X, center_y + ROW_HEIGHT),
                    );
                    frame.stroke(
                        &connector,
                        canvas::Stroke::default()
                            .with_color(theme::TEXT_TERTIARY)
                            .with_width(1.0),
                    );
                }

                let node = canvas::Path::circle(Point::new(GRAPH_X, center_y), 4.5);
                frame.fill(&node, theme::GIT_ADDED);

                frame.fill_text(canvas::Text {
                    content: commit.short_id.clone(),
                    position: Point::new(48.0, center_y - 10.0),
                    color: theme::TEXT_SECONDARY,
                    size: Pixels::from(12.0),
                    line_height: iced::widget::text::LineHeight::default(),
                    font: Font::MONOSPACE,
                    align_x: iced::alignment::Horizontal::Left.into(),
                    align_y: alignment::Vertical::Top,
                    shaping: iced::widget::text::Shaping::Basic,
                    max_width: 90.0,
                });

                frame.fill_text(canvas::Text {
                    content: commit.summary.clone(),
                    position: Point::new(120.0, center_y - 10.0),
                    color: theme::TEXT_PRIMARY,
                    size: Pixels::from(14.0),
                    line_height: iced::widget::text::LineHeight::default(),
                    font: Font::DEFAULT,
                    align_x: iced::alignment::Horizontal::Left.into(),
                    align_y: alignment::Vertical::Top,
                    shaping: iced::widget::text::Shaping::Basic,
                    max_width: (bounds.width - 140.0).max(120.0),
                });
            }
        })]
    }
}
