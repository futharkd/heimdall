use super::input::CustomFirewallRule;
use anyhow::{anyhow, Result};

pub fn validate_custom_rule(rule: &CustomFirewallRule) -> Result<()> {
    // Port is u16, so it's always in range 0-65535. No validation needed.

    // Validate protocol
    match rule.protocol.as_str() {
        "tcp" | "udp" | "both" => Ok(()),
        _ => Err(anyhow!(
            "invalid protocol '{}'. must be tcp, udp, or both",
            rule.protocol
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_custom_rule_valid() {
        let rule = CustomFirewallRule {
            port: 8080,
            protocol: "tcp".to_string(),
        };
        assert!(validate_custom_rule(&rule).is_ok());

        let rule_udp = CustomFirewallRule {
            port: 53,
            protocol: "udp".to_string(),
        };
        assert!(validate_custom_rule(&rule_udp).is_ok());

        let rule_both = CustomFirewallRule {
            port: 5000,
            protocol: "both".to_string(),
        };
        assert!(validate_custom_rule(&rule_both).is_ok());
    }

    #[test]
    fn test_validate_custom_rule_invalid_protocol() {
        let rule = CustomFirewallRule {
            port: 8080,
            protocol: "http".to_string(),
        };
        assert!(validate_custom_rule(&rule).is_err());
    }
}
