use termcolor::{ColorChoice, ColorSpec, WriteColor, StandardStream};
use std::io::Write;
use crate::tui::theme::PALETTE;

#[derive(Debug, Clone)]
pub enum DiffLine {
    Same(String),
    Added(String),
    Removed(String),
    Header(String),
}

pub struct DiffBlock {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

pub struct DiffRenderer {
    pub blocks: Vec<DiffBlock>,
    pub context_lines: usize,
}

impl Default for DiffRenderer {
    fn default() -> Self {
        Self { blocks: vec![], context_lines: 3 }
    }
}

impl DiffRenderer {
    pub fn from_unified(diff_text: &str) -> Self {
        let mut blocks = vec![];
        let mut current_lines: Vec<DiffLine> = vec![];
        let mut current_header = String::new();
        let mut in_hunk = false;

        for line in diff_text.lines() {
            if line.starts_with("--- ") || line.starts_with("+++ ") {
                if in_hunk && !current_lines.is_empty() {
                    blocks.push(DiffBlock { header: current_header.clone(), lines: current_lines.clone() });
                    current_lines.clear();
                }
                current_header = line.to_string();
                continue;
            }
            if line.starts_with("@@") {
                if in_hunk && !current_lines.is_empty() {
                    blocks.push(DiffBlock { header: current_header.clone(), lines: current_lines.clone() });
                    current_lines.clear();
                }
                current_lines.push(DiffLine::Header(line.to_string()));
                in_hunk = true;
                continue;
            }
            if line.starts_with("+") {
                current_lines.push(DiffLine::Added(line[1..].to_string()));
            } else if line.starts_with("-") {
                current_lines.push(DiffLine::Removed(line[1..].to_string()));
            } else if line.starts_with(' ') || line.is_empty() {
                current_lines.push(DiffLine::Same(line.to_string()));
            }
        }
        if in_hunk && !current_lines.is_empty() {
            blocks.push(DiffBlock { header: current_header.clone(), lines: current_lines });
        }

        Self { blocks, context_lines: 3 }
    }

    pub fn from_strings(before: &str, after: &str, file_path: &str) -> Self {
        let diff = simple_diff(before, after);
        Self {
            blocks: vec![DiffBlock { header: format!("─── {} ───", file_path), lines: diff }],
            context_lines: 3,
        }
    }

    pub fn render(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut w = StandardStream::stdout(ColorChoice::Auto);

        for block in &self.blocks {
            let mut header_spec = ColorSpec::new();
            header_spec.set_fg(Some(PALETTE.accent));
            header_spec.set_bold(true);
            w.set_color(&header_spec)?;
            if !block.header.is_empty() {
                writeln!(&mut w, "{}", block.header)?;
            }
            w.reset()?;

            for line in &block.lines {
                match line {
                    DiffLine::Header(h) => {
                        let mut h_spec = ColorSpec::new();
                        h_spec.set_fg(Some(PALETTE.cyan));
                        h_spec.set_bold(true);
                        w.set_color(&h_spec)?;
                        writeln!(&mut w, "{}", h)?;
                        w.reset()?;
                    }
                    DiffLine::Added(s) => {
                        let mut add_spec = ColorSpec::new();
                        add_spec.set_fg(Some(PALETTE.green));
                        w.set_color(&add_spec)?;
                        writeln!(&mut w, "+ {}", s)?;
                        w.reset()?;
                    }
                    DiffLine::Removed(s) => {
                        let mut rem_spec = ColorSpec::new();
                        rem_spec.set_fg(Some(PALETTE.red));
                        w.set_color(&rem_spec)?;
                        writeln!(&mut w, "- {}", s)?;
                        w.reset()?;
                    }
                    DiffLine::Same(s) => {
                        let mut dim_spec = ColorSpec::new();
                        dim_spec.set_fg(Some(PALETTE.dim));
                        w.set_color(&dim_spec)?;
                        writeln!(&mut w, "  {}", s)?;
                        w.reset()?;
                    }
                }
            }
        }

        Ok(())
    }
}

fn simple_diff(before: &str, after: &str) -> Vec<DiffLine> {
    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();

    let mut result = vec![];
    let max = before_lines.len().max(after_lines.len());

    for i in 0..max {
        let b = before_lines.get(i).copied().unwrap_or("");
        let a = after_lines.get(i).copied().unwrap_or("");

        if b == a {
            if !b.is_empty() || !result.is_empty() {
                result.push(DiffLine::Same(b.to_string()));
            }
        } else {
            if i < before_lines.len() {
                result.push(DiffLine::Removed(b.to_string()));
            }
            if i < after_lines.len() {
                result.push(DiffLine::Added(a.to_string()));
            }
        }
    }

    result
}
