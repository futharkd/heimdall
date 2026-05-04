use serde_json::{Value, json};

pub fn generate_daemon_json(
    log_driver: Option<&str>,
    registry_mirrors: &[String],
) -> anyhow::Result<String> {
    let mut config = json!({});

    if let Some(driver) = log_driver {
        config["log-driver"] = Value::String(driver.to_string());
    }

    if !registry_mirrors.is_empty() {
        config["registry-mirrors"] = Value::Array(
            registry_mirrors
                .iter()
                .map(|m| Value::String(m.clone()))
                .collect(),
        );
    }

    let json_str = serde_json::to_string_pretty(&config)?;
    Ok(json_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_daemon_json_with_log_driver() {
        let result = generate_daemon_json(Some("json-file"), &[]).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["log-driver"], "json-file");
    }

    #[test]
    fn generate_daemon_json_with_registry_mirrors() {
        let mirrors = vec![
            "https://mirror1.example.com".to_string(),
            "https://mirror2.example.com".to_string(),
        ];
        let result = generate_daemon_json(None, &mirrors).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["registry-mirrors"][0], "https://mirror1.example.com");
        assert_eq!(parsed["registry-mirrors"][1], "https://mirror2.example.com");
    }

    #[test]
    fn generate_daemon_json_with_both() {
        let mirrors = vec!["https://mirror.example.com".to_string()];
        let result = generate_daemon_json(Some("json-file"), &mirrors).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["log-driver"], "json-file");
        assert_eq!(parsed["registry-mirrors"][0], "https://mirror.example.com");
    }

    #[test]
    fn generate_daemon_json_empty() {
        let result = generate_daemon_json(None, &[]).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed, json!({}));
    }
}
