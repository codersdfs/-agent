use termcolor::{ColorChoice, ColorSpec, WriteColor, StandardStream};
use std::io::Write;
use crate::tui::theme::PALETTE;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StepStatus {
    Pending,
    Active,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct TimelineStep {
    pub label: &'static str,
    pub status: StepStatus,
    pub detail: Option<String>,
}

pub struct Timeline {
    pub steps: Vec<TimelineStep>,
    pub title: Option<String>,
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            steps: vec![
                TimelineStep { label: "Plan", status: StepStatus::Pending, detail: None },
                TimelineStep { label: "Build", status: StepStatus::Pending, detail: None },
                TimelineStep { label: "Review", status: StepStatus::Pending, detail: None },
                TimelineStep { label: "Gate", status: StepStatus::Pending, detail: None },
            ],
            title: None,
        }
    }
}

impl Timeline {
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

        let mut dim_spec = ColorSpec::new();
        dim_spec.set_fg(Some(PALETTE.dim));

        let mut white_bold = ColorSpec::new();
        white_bold.set_fg(Some(PALETTE.white));
        white_bold.set_bold(true);

        for (i, step) in self.steps.iter().enumerate() {
            let (icon, fg) = match step.status {
                StepStatus::Completed => ("●", PALETTE.green),
                StepStatus::Active => ("◉", PALETTE.accent),
                StepStatus::Failed => ("✗", PALETTE.red),
                StepStatus::Skipped => ("○", PALETTE.muted),
                StepStatus::Pending => ("○", PALETTE.muted),
            };

            let is_active = step.status == StepStatus::Active;
            let mut step_spec = ColorSpec::new();
            step_spec.set_fg(Some(fg));
            step_spec.set_bold(is_active);
            w.set_color(&step_spec)?;
            write!(&mut w, " {}", icon)?;
            w.set_color(&white_bold)?;
            write!(&mut w, " {}", step.label)?;

            if let Some(ref d) = step.detail {
                w.set_color(&dim_spec)?;
                write!(&mut w, " ({})", d)?;
            }

            if i < self.steps.len() - 1 {
                let connect_fg = if self.steps[i + 1].status != StepStatus::Pending {
                    PALETTE.green
                } else {
                    PALETTE.muted
                };
                let mut connect_spec = ColorSpec::new();
                connect_spec.set_fg(Some(connect_fg));
                w.set_color(&connect_spec)?;
                write!(&mut w, " —")?;
            }

            w.reset()?;
        }
        writeln!(&mut w)?;

        Ok(())
    }
}
