use comfy_table::*;
use termcolor::{Color, ColorChoice, ColorSpec, WriteColor, StandardStream};
use std::io::Write;
use crate::tui::theme::{self, PALETTE};

#[derive(Debug, Clone)]
pub struct GateTableRow {
    pub category: String,
    pub message: String,
    pub line: Option<u32>,
    pub tool_hint: Option<String>,
}

pub struct GateTable {
    pub title: Option<String>,
    pub score: u32,
    pub threshold: u32,
    pub passed: bool,
    pub violations: Vec<GateTableRow>,
}

impl GateTable {
    pub fn render(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut w = StandardStream::stdout(ColorChoice::Auto);

        if let Some(ref t) = self.title {
            let mut title_spec = ColorSpec::new();
            title_spec.set_fg(Some(PALETTE.accent));
            title_spec.set_bold(true);
            w.set_color(&title_spec)?;
            writeln!(&mut w, "{}", t)?;
            w.reset()?;
        }

        let sc = theme::score_color(self.score, self.threshold);
        let mut score_spec = ColorSpec::new();
        score_spec.set_fg(Some(sc));
        score_spec.set_bold(true);
        w.set_color(&score_spec)?;
        let outcome = if self.passed { "PASS" } else { "FAIL" };
        writeln!(&mut w, "Score: {} / {} — {}", self.score, self.threshold, outcome)?;
        w.reset()?;

        if self.violations.is_empty() {
            let mut ok_spec = ColorSpec::new();
            ok_spec.set_fg(Some(PALETTE.green));
            w.set_color(&ok_spec)?;
            writeln!(&mut w, "  No violations.")?;
            w.reset()?;
            return Ok(());
        }

        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Category").add_attribute(Attribute::Bold),
                Cell::new("Message").add_attribute(Attribute::Bold),
                Cell::new("Line").add_attribute(Attribute::Bold),
                Cell::new("Hint").add_attribute(Attribute::Bold),
            ]);

        for v in &self.violations {
            let cat_color = theme::category_color(&v.category);
            let ansi_code = match cat_color {
                Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
                _ => "\x1b[1m".to_string(),
            };
            let colored_cat = format!("{}{}\x1b[0m", ansi_code, v.category);

            let line_str = v.line.map(|l| l.to_string()).unwrap_or_default();
            let hint = v.tool_hint.as_deref().unwrap_or("");

            table.add_row(vec![
                Cell::new(&colored_cat),
                Cell::new(&v.message),
                Cell::new(&line_str),
                Cell::new(hint),
            ]);
        }

        writeln!(&mut w, "{}", table)?;

        let mut dim_spec = ColorSpec::new();
        dim_spec.set_fg(Some(PALETTE.dim));
        w.set_color(&dim_spec)?;
        writeln!(&mut w, "  {} violation(s) found", self.violations.len())?;
        w.reset()?;

        Ok(())
    }
}
