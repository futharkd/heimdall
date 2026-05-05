use crate::features::bootstrap::infisical::input::BootstrapInfisicalConfig;

pub fn agent_yaml(config: &BootstrapInfisicalConfig) -> String {
    let creds_dir = format!("{}/.infisical", config.secrets_dir);

    let templates = if config.folders.is_empty() {
        format!(
            r#"  - template-content: |
      {{{{- with listSecretsByProjectSlug "{}" "{}" "/{}" `{{"recursive": false, "expandSecretReferences": true}}` }}}}
      {{{{- range . }}}}
      {{{{ .Key }}}}={{{{ .Value }}}}
      {{{{- end }}}}
      {{{{- end }}}}
    destination-path: {}/.env
    config:
      polling-interval: 60s"#,
            config.project_slug, config.environment, config.node_name, config.secrets_dir
        )
    } else {
        let mut templates_str = format!(
            r#"  - template-content: |
      {{{{- with listSecretsByProjectSlug "{}" "{}" "/{}" `{{"recursive": false, "expandSecretReferences": true}}` }}}}
      {{{{- range . }}}}
      {{{{ .Key }}}}={{{{ .Value }}}}
      {{{{- end }}}}
      {{{{- end }}}}
    destination-path: {}/.env
    config:
      polling-interval: 60s"#,
            config.project_slug, config.environment, config.node_name, config.secrets_dir
        );

        for folder in &config.folders {
            templates_str.push_str(&format!(
                r#"

  - template-content: |
      {{{{- with listSecretsByProjectSlug "{}" "{}" "/{}/{}" `{{"recursive": false, "expandSecretReferences": true}}` }}}}
      {{{{- range . }}}}
      {{{{ .Key }}}}={{{{ .Value }}}}
      {{{{- end }}}}
      {{{{- end }}}}
    destination-path: {}/{}/.env
    config:
      polling-interval: 60s"#,
                config.project_slug,
                config.environment,
                config.node_name,
                folder,
                config.secrets_dir,
                folder
            ));
        }

        templates_str
    };

    format!(
        r#"infisical:
  address: "{}"

auth:
  type: "universal-auth"
  config:
    client-id: "{}/client-id"
    client-secret: "{}/client-secret"
    remove_client_secret_on_read: false

templates:
{}
"#,
        config.address, creds_dir, creds_dir, templates
    )
}

pub fn systemd_unit(config_path: &str) -> String {
    format!(
        r#"[Unit]
Description=Infisical Agent
After=network.target

[Service]
Type=simple
ExecStart=/usr/bin/infisical agent --config {}
Restart=on-failure
RestartSec=5s
User=root

[Install]
WantedBy=multi-user.target
"#,
        config_path
    )
}
