use termcolor::{ColorChoice, ColorSpec, WriteColor, StandardStream};
use std::io::{Write, stdin, stdout};
use crate::tui::theme::PALETTE;

pub struct PermissionPrompt {
    pub tool: String,
    pub file_path: Option<String>,
    pub reason: String,
    pub step_id: u32,
    pub step_description: String,
    pub timeout_secs: u64,
}

impl PermissionPrompt {
    pub fn render(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let mut w = StandardStream::stdout(ColorChoice::Auto);

        let mut border_spec = ColorSpec::new();
        border_spec.set_fg(Some(PALETTE.yellow));
        w.set_color(&border_spec)?;
        writeln!(&mut w, "{}", "┌─ Permission Required ")?;
        w.reset()?;

        let mut dim_spec = ColorSpec::new();
        dim_spec.set_fg(Some(PALETTE.dim));
        let mut white_spec = ColorSpec::new();
        white_spec.set_fg(Some(PALETTE.white));
        white_spec.set_bold(true);

        w.set_color(&dim_spec)?;
        write!(&mut w, "│ Step #{}: ", self.step_id)?;
        w.set_color(&white_spec)?;
        writeln!(&mut w, "{}", self.step_description)?;

        w.set_color(&dim_spec)?;
        write!(&mut w, "│ Tool: ")?;
        let mut tool_spec = ColorSpec::new();
        tool_spec.set_fg(Some(PALETTE.accent));
        tool_spec.set_bold(true);
        w.set_color(&tool_spec)?;
        writeln!(&mut w, "{}", self.tool)?;

        if let Some(ref fp) = self.file_path {
            w.set_color(&dim_spec)?;
            write!(&mut w, "│ File: ")?;
            let mut file_spec = ColorSpec::new();
            file_spec.set_fg(Some(PALETTE.cyan));
            w.set_color(&file_spec)?;
            writeln!(&mut w, "{}", fp)?;
        }

        w.set_color(&dim_spec)?;
        write!(&mut w, "│ Reason: ")?;
        w.set_color(&white_spec)?;
        writeln!(&mut w, "{}", self.reason)?;

        w.set_color(&border_spec)?;
        writeln!(&mut w, "{}", "└")?;
        w.reset()?;

        let mut prompt_spec = ColorSpec::new();
        prompt_spec.set_fg(Some(PALETTE.green));
        prompt_spec.set_bold(true);
        w.set_color(&prompt_spec)?;
        write!(&mut w, "Allow? (y/N) [{}s]: ", self.timeout_secs)?;
        w.reset()?;
        let _ = stdout().flush();

        let mut input = String::new();
        match stdin().read_line(&mut input) {
            Ok(_) => {
                let trimmed = input.trim().to_lowercase();
                Ok(trimmed == "y" || trimmed == "yes")
            }
            Err(_) => Ok(false),
        }
    }
}
