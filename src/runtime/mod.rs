mod privilege_reexec;

pub use privilege_reexec::maybe_privilege_reexec;

use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Success,
    #[allow(dead_code)]
    Warning,
    Failure,
}

impl ExitStatus {
    pub fn code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::Warning => 0,
            Self::Failure => 1,
        }
    }
}

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
