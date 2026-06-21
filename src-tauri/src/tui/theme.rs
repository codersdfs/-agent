use termcolor::{Color, ColorSpec};

pub struct Palette {
    pub accent: Color,
    pub accent_dim: Color,
    pub green: Color,
    pub red: Color,
    pub yellow: Color,
    pub blue: Color,
    pub cyan: Color,
    pub white: Color,
    pub dim: Color,
    pub muted: Color,
    pub surface: Color,
}

pub const PALETTE: Palette = Palette {
    accent: Color::Rgb(160, 120, 255),
    accent_dim: Color::Rgb(109, 59, 215),
    green: Color::Rgb(74, 222, 128),
    red: Color::Rgb(248, 113, 113),
    yellow: Color::Rgb(250, 204, 21),
    blue: Color::Rgb(96, 165, 250),
    cyan: Color::Rgb(34, 211, 238),
    white: Color::Rgb(226, 232, 240),
    dim: Color::Rgb(148, 163, 184),
    muted: Color::Rgb(100, 116, 139),
    surface: Color::Rgb(30, 41, 59),
};

pub fn spec(fg: Option<Color>, bold: bool) -> ColorSpec {
    let mut s = ColorSpec::new();
    s.set_fg(fg);
    s.set_bold(bold);
    s
}

pub fn status_color(status: &str) -> Color {
    match status {
        "idle" | "completed" => PALETTE.green,
        "running" | "building" | "planning" | "reviewing" => PALETTE.accent,
        "retrying" => PALETTE.yellow,
        "failed" | "error" => PALETTE.red,
        _ => PALETTE.dim,
    }
}

pub fn score_color(score: u32, threshold: u32) -> Color {
    if score >= threshold { PALETTE.green }
    else if score >= threshold / 2 { PALETTE.yellow }
    else { PALETTE.red }
}

pub fn category_color(cat: &str) -> Color {
    match cat.to_lowercase().as_str() {
        "golden" => PALETTE.red,
        "structural" => PALETTE.blue,
        "taste" => PALETTE.yellow,
        "repeated" => PALETTE.cyan,
        _ => PALETTE.dim,
    }
}
