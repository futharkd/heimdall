use std::path::Path;

use super::super::report::{CheckStatus, DoctorCheck};

const KOMODO_COMPOSE_PATH: &str = "/etc/heimdall/komodo/compose.yaml";

pub fn contribute() -> Vec<DoctorCheck> {
    let p = Path::new(KOMODO_COMPOSE_PATH);
    let (status, detail) = if p.is_file() {
        (CheckStatus::Pass, format!("found {}", p.display()))
    } else {
        (
            CheckStatus::Warn,
            format!("{} missing (default Komodo compose path)", p.display()),
        )
    };
    vec![DoctorCheck {
        id: "bootstrap_komodo",
        description: "Komodo compose bundle",
        status,
        detail,
    }]
}
