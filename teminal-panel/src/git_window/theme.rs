use iced::Color;

pub const BG_PRIMARY: Color = Color::from_rgb(0.039, 0.055, 0.078);
pub const BG_SECONDARY: Color = Color::from_rgb(0.059, 0.078, 0.098);
pub const BG_TERTIARY: Color = Color::from_rgb(0.082, 0.102, 0.129);
pub const BG_ELEVATED: Color = Color::from_rgb(0.102, 0.122, 0.157);

pub const TEXT_PRIMARY: Color = Color::from_rgb(0.902, 0.929, 0.953);
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.545, 0.580, 0.620);
pub const TEXT_TERTIARY: Color = Color::from_rgb(0.431, 0.463, 0.506);

pub const GIT_ADDED: Color = Color::from_rgb(0.247, 0.725, 0.314);
pub const GIT_MODIFIED: Color = Color::from_rgb(0.824, 0.600, 0.133);
pub const GIT_DELETED: Color = Color::from_rgb(0.973, 0.318, 0.286);

pub const BRANCH_COLORS: [Color; 5] = [
    Color::from_rgb(0.345, 0.651, 1.0),
    Color::from_rgb(0.247, 0.725, 0.314),
    Color::from_rgb(0.824, 0.600, 0.133),
    Color::from_rgb(0.737, 0.549, 1.0),
    Color::from_rgb(1.0, 0.482, 0.447),
];
