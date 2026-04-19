use std::borrow::Cow;
use std::io::IsTerminal;

use clap::ValueEnum;

/// Global `--color` (see also `NO_COLOR` in [`Style::for_human`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum ColorArg {
    /// Color when stderr is a TTY and `NO_COLOR` is unset.
    #[default]
    Auto,
    Always,
    Never,
}

/// Minimal ANSI styling for human reports (status tokens and headings only).
#[derive(Debug, Clone, Copy)]
pub struct Style {
    ansi: bool,
}

impl Style {
    /// Human report formatting: respects `color`, `NO_COLOR`, and TTY detection.
    pub fn for_human(color: ColorArg) -> Self {
        let no_color = std::env::var_os("NO_COLOR").is_some();
        let tty = std::io::stderr().is_terminal();
        let ansi = !no_color
            && match color {
                ColorArg::Auto => tty,
                ColorArg::Always => true,
                ColorArg::Never => false,
            };
        Self { ansi }
    }

    /// JSON / machine paths: never emit escape sequences.
    pub const fn plain() -> Self {
        Self { ansi: false }
    }

    fn wrap<'a>(&self, codes: &str, text: &'a str) -> Cow<'a, str> {
        if !self.ansi {
            return Cow::Borrowed(text);
        }
        Cow::Owned(format!("{codes}{text}\x1b[0m"))
    }

    pub fn bold(&self, text: &str) -> String {
        self.wrap("\x1b[1m", text).into_owned()
    }

    pub fn dim(&self, text: &str) -> String {
        self.wrap("\x1b[2m", text).into_owned()
    }

    pub fn green(&self, text: &str) -> String {
        self.wrap("\x1b[32m", text).into_owned()
    }

    pub fn yellow(&self, text: &str) -> String {
        self.wrap("\x1b[33m", text).into_owned()
    }

    pub fn red(&self, text: &str) -> String {
        self.wrap("\x1b[31m", text).into_owned()
    }

    pub fn cyan(&self, text: &str) -> String {
        self.wrap("\x1b[36m", text).into_owned()
    }

    /// Short status token for operation lines (PLAN / OK / FAIL / SKIP).
    pub fn status_token(&self, label: &str, tone: StatusTone) -> String {
        let raw = format!("[{label}]");
        match tone {
            StatusTone::Planned => self.yellow(&raw),
            StatusTone::Ok => self.green(&raw),
            StatusTone::Warn => self.yellow(&raw),
            StatusTone::Fail => self.red(&raw),
            StatusTone::Skip => self.dim(&raw),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StatusTone {
    Planned,
    Ok,
    Warn,
    Fail,
    Skip,
}
