use ratatui::style::{Color, Modifier, Style};

use super::TermBuf;

impl TermBuf {
    pub(crate) fn handle_sgr(&mut self, params: &[u16]) {
        if params.is_empty() {
            self.current_style = Style::default();
            return;
        }
        let mut i = 0;
        while i < params.len() {
            i += self.apply_sgr_code(params, i);
        }
    }

    /// Apply one SGR code at `params[i]`, return how many params consumed.
    fn apply_sgr_code(&mut self, params: &[u16], i: usize) -> usize {
        let Some(&code) = params.get(i) else { return 1 };
        match code {
            0 => self.current_style = Style::default(),
            1 => self.current_style = self.current_style.add_modifier(Modifier::BOLD),
            3 => {
                self.current_style = self.current_style.add_modifier(Modifier::ITALIC);
            }
            4 => {
                self.current_style = self.current_style.add_modifier(Modifier::UNDERLINED);
            }
            7 => {
                self.current_style = self.current_style.add_modifier(Modifier::REVERSED);
            }
            22 => {
                self.current_style = self.current_style.remove_modifier(Modifier::BOLD);
            }
            23 => {
                self.current_style = self.current_style.remove_modifier(Modifier::ITALIC);
            }
            24 => {
                self.current_style = self.current_style.remove_modifier(Modifier::UNDERLINED);
            }
            27 => {
                self.current_style = self.current_style.remove_modifier(Modifier::REVERSED);
            }
            30..=37 => {
                self.current_style = self.current_style.fg(ansi_color(code - 30));
            }
            38 => {
                let mut j = i + 1;
                if let Some(c) = parse_extended_color(params, &mut j) {
                    self.current_style = self.current_style.fg(c);
                }
                return j - i;
            }
            39 => self.current_style = self.current_style.fg(Color::Reset),
            40..=47 => {
                self.current_style = self.current_style.bg(ansi_color(code - 40));
            }
            48 => {
                let mut j = i + 1;
                if let Some(c) = parse_extended_color(params, &mut j) {
                    self.current_style = self.current_style.bg(c);
                }
                return j - i;
            }
            49 => self.current_style = self.current_style.bg(Color::Reset),
            90..=97 => {
                self.current_style = self.current_style.fg(ansi_bright_color(code - 90));
            }
            100..=107 => {
                self.current_style = self.current_style.bg(ansi_bright_color(code - 100));
            }
            _ => {}
        }
        1
    }
}

const fn ansi_color(n: u16) -> Color {
    match n {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        _ => Color::White,
    }
}

const fn ansi_bright_color(n: u16) -> Color {
    match n {
        0 => Color::DarkGray,
        1 => Color::LightRed,
        2 => Color::LightGreen,
        3 => Color::LightYellow,
        4 => Color::LightBlue,
        5 => Color::LightMagenta,
        6 => Color::LightCyan,
        _ => Color::White,
    }
}

// params[*i] accesses are guarded by `*i >= params.len()` / `*i + 2 < params.len()` checks.
fn parse_extended_color(params: &[u16], i: &mut usize) -> Option<Color> {
    let &code = params.get(*i)?;
    match code {
        5 => {
            *i += 1;
            let &n = params.get(*i)?;
            *i += 1;
            Some(Color::Indexed(u8::try_from(n).unwrap_or(u8::MAX)))
        }
        2 => {
            *i += 1;
            if *i + 2 < params.len() {
                let r = u8::try_from(params.get(*i).copied().unwrap_or(0)).unwrap_or(0);
                let g = u8::try_from(params.get(*i + 1).copied().unwrap_or(0)).unwrap_or(0);
                let b = u8::try_from(params.get(*i + 2).copied().unwrap_or(0)).unwrap_or(0);
                *i += 3;
                Some(Color::Rgb(r, g, b))
            } else {
                None
            }
        }
        _ => None,
    }
}
