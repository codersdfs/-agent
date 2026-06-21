use termcolor::{ColorChoice, ColorSpec, WriteColor, StandardStream};
use std::io::Write;
use crate::tui::theme::{self, PALETTE};

pub struct StatusBar {
    pub status: String,
    pub model: String,
    pub provider: String,
    pub score: Option<u32>,
    pub pass_threshold: u32,
    pub gate_passed: Option<bool>,
    pub retry: Option<(u8, u8)>,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self {
            status: "idle".into(),
            model: "—".into(),
            provider: "—".into(),
            score: None,
            pass_threshold: 80,
            gate_passed: None,
            retry: None,
        }
    }
}

impl StatusBar {
    pub fn render(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut w = StandardStream::stdout(ColorChoice::Auto);

        write!(&mut w, "\r")?;

        let mut bg_spec = ColorSpec::new();
        bg_spec.set_bg(Some(PALETTE.surface));
        w.set_color(&bg_spec)?;

        let status_c = theme::status_color(&self.status.to_lowercase());
        let mut status_spec = ColorSpec::new();
        status_spec.set_fg(Some(status_c));
        status_spec.set_bold(true);
        w.set_color(&status_spec)?;
        write!(&mut w, " Ω ")?;

        let mut dim_spec = ColorSpec::new();
        dim_spec.set_fg(Some(PALETTE.dim));
        w.set_color(&dim_spec)?;
        write!(&mut w, "│ ")?;

        let mut white_bold = ColorSpec::new();
        white_bold.set_fg(Some(PALETTE.white));
        white_bold.set_bold(true);
        w.set_color(&white_bold)?;
        let status_upper: String = self.status.chars().take(12).collect();
        write!(&mut w, " {:>12}", status_upper)?;

        w.set_color(&dim_spec)?;
        write!(&mut w, " │ ")?;

        w.set_color(&white_bold)?;
        write!(&mut w, "{}", self.model)?;

        w.set_color(&dim_spec)?;
        write!(&mut w, " @{}", self.provider)?;

        if let Some(score) = self.score {
            let sc = theme::score_color(score, self.pass_threshold);
            let mut score_spec = ColorSpec::new();
            score_spec.set_fg(Some(sc));
            score_spec.set_bold(true);
            w.set_color(&score_spec)?;
            write!(&mut w, " │ score: {}", score)?;
        }

        if let Some(passed) = self.gate_passed {
            let gc = if passed { PALETTE.green } else { PALETTE.red };
            let mut gate_spec = ColorSpec::new();
            gate_spec.set_fg(Some(gc));
            w.set_color(&gate_spec)?;
            write!(&mut w, " │ gate: {}", if passed { "PASS" } else { "FAIL" })?;
        }

        if let Some((cur, max)) = self.retry {
            let mut retry_spec = ColorSpec::new();
            retry_spec.set_fg(Some(PALETTE.yellow));
            w.set_color(&retry_spec)?;
            write!(&mut w, " │ retry: {}/{}", cur, max)?;
        }

        w.reset()?;
        writeln!(&mut w)?;

        Ok(())
    }
}
