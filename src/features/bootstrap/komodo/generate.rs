use crate::features::bootstrap::komodo::input::BootstrapKomodoConfig;

pub fn compose_yaml_core(config: &BootstrapKomodoConfig) -> String {
    format!(
        r#"################################
# 🦎 KOMODO COMPOSE - MONGO 🦎 #
################################

version: '3.8'

services:
  mongo:
    image: mongo
    labels:
      komodo.skip: ""
    command: --quiet --wiredTigerCacheSizeGB 0.25
    restart: unless-stopped
    volumes:
      - mongo-data:/data/db
      - mongo-config:/data/configdb
    environment:
      MONGO_INITDB_ROOT_USERNAME: ${{KOMODO_DATABASE_USERNAME}}
      MONGO_INITDB_ROOT_PASSWORD: ${{KOMODO_DATABASE_PASSWORD}}

  core:
    image: ghcr.io/moghtech/komodo-core:${{COMPOSE_KOMODO_IMAGE_TAG:-2}}
    init: true
    restart: unless-stopped
    depends_on:
      - mongo
    ports:
      - "{port}:9120"
    env_file: ./compose.env
    environment:
      KOMODO_DATABASE_ADDRESS: mongo:27017
    volumes:
      - keys:/config/keys
      - ${{COMPOSE_KOMODO_BACKUPS_PATH}}:/backups

  periphery:
    image: ghcr.io/moghtech/komodo-periphery:${{COMPOSE_KOMODO_IMAGE_TAG:-2}}
    init: true
    restart: unless-stopped
    depends_on:
      - core
    env_file: ./compose.env
    volumes:
      - keys:/config/keys
      - /var/run/docker.sock:/var/run/docker.sock
      - /proc:/proc
      - ${{PERIPHERY_ROOT_DIRECTORY:-/etc/komodo}}:${{PERIPHERY_ROOT_DIRECTORY:-/etc/komodo}}

volumes:
  mongo-data:
  mongo-config:
  keys:
"#,
        port = config.port
    )
}

pub fn compose_yaml_periphery(_config: &BootstrapKomodoConfig) -> String {
    format!(
        r#"################################
# 🦎 KOMODO PERIPHERY COMPOSE 🦎 #
################################

version: '3.8'

services:
  periphery:
    image: ghcr.io/moghtech/komodo-periphery:${{COMPOSE_KOMODO_IMAGE_TAG:-2}}
    init: true
    restart: unless-stopped
    env_file: ./compose.env
    volumes:
      - ./keys:/config/keys
      - /var/run/docker.sock:/var/run/docker.sock
      - /proc:/proc
      - ${{PERIPHERY_ROOT_DIRECTORY:-/etc/komodo}}:${{PERIPHERY_ROOT_DIRECTORY:-/etc/komodo}}
"#
    )
}

pub fn compose_env_core(config: &BootstrapKomodoConfig) -> String {
    let core_public_key_ref = "file:/config/keys/core.pub";
    let periphery_public_key_ref = "file:/config/keys/periphery.pub";
    let host_ref = config
        .host
        .as_deref()
        .unwrap_or("https://komodo.example.com");

    format!(
        "COMPOSE_KOMODO_IMAGE_TAG={image_tag}
COMPOSE_KOMODO_BACKUPS_PATH={backups_path}

KOMODO_DATABASE_USERNAME={db_username}
KOMODO_DATABASE_PASSWORD={db_password}

TZ=Etc/UTC

KOMODO_HOST={host}
KOMODO_TITLE={title}

KOMODO_PERIPHERY_PUBLIC_KEY={periphery_public_key_ref}

KOMODO_LOCAL_AUTH=true
KOMODO_INIT_ADMIN_USERNAME={admin_username}
KOMODO_INIT_ADMIN_PASSWORD={admin_password}

KOMODO_FIRST_SERVER_NAME={first_server_name}

KOMODO_DISABLE_CONFIRM_DIALOG=false

KOMODO_DISABLE_INIT_RESOURCES=false

KOMODO_WEBHOOK_SECRET={webhook_secret}
KOMODO_JWT_SECRET={jwt_secret}
KOMODO_JWT_TTL=1-day

KOMODO_MONITORING_INTERVAL=15-sec
KOMODO_RESOURCE_POLL_INTERVAL=1-hr

KOMODO_DISABLE_USER_REGISTRATION=false
KOMODO_ENABLE_NEW_USERS=false
KOMODO_DISABLE_NON_ADMIN_CREATE=false
KOMODO_TRANSPARENT_MODE=false

KOMODO_OIDC_ENABLED=false

KOMODO_GITHUB_OAUTH_ENABLED=false

KOMODO_GOOGLE_OAUTH_ENABLED=false

KOMODO_LOGGING_PRETTY=false
KOMODO_PRETTY_STARTUP_CONFIG=false

PERIPHERY_CORE_ADDRESS=ws://core:9120
PERIPHERY_CONNECT_AS={first_server_name}
PERIPHERY_CORE_PUBLIC_KEYS={core_public_key_ref}

PERIPHERY_ROOT_DIRECTORY={periphery_root}

PERIPHERY_DISABLE_TERMINALS=false
PERIPHERY_DISABLE_CONTAINER_TERMINALS=false

PERIPHERY_LOGGING_PRETTY=false
PERIPHERY_PRETTY_STARTUP_CONFIG=false
",
        image_tag = config.image_tag,
        backups_path = config.backups_path,
        db_username = config.db_username,
        db_password = config.db_password,
        host = host_ref,
        title = config.title,
        periphery_public_key_ref = periphery_public_key_ref,
        admin_username = config.admin_username,
        admin_password = config.admin_password,
        first_server_name = config.first_server_name,
        webhook_secret = generate_secret_inline(),
        jwt_secret = generate_secret_inline(),
        periphery_root = config.periphery_root,
    )
}

pub fn compose_env_periphery(config: &BootstrapKomodoConfig) -> String {
    let core_address = config.core_address.as_deref().unwrap_or("ws://core:9120");
    let core_public_key_ref = if config.core_public_key_content.is_some() {
        "file:/config/keys/core.pub"
    } else {
        "file:/config/keys/core.pub"
    };

    format!(
        "COMPOSE_KOMODO_IMAGE_TAG={image_tag}

TZ=Etc/UTC

PERIPHERY_CORE_ADDRESS={core_address}
PERIPHERY_CONNECT_AS={connect_as}
PERIPHERY_CORE_PUBLIC_KEYS={core_public_key_ref}

PERIPHERY_ROOT_DIRECTORY={periphery_root}

PERIPHERY_DISABLE_TERMINALS=false
PERIPHERY_DISABLE_CONTAINER_TERMINALS=false

PERIPHERY_LOGGING_PRETTY=false
PERIPHERY_PRETTY_STARTUP_CONFIG=false
",
        image_tag = config.image_tag,
        core_address = core_address,
        connect_as = config.connect_as,
        core_public_key_ref = core_public_key_ref,
        periphery_root = config.periphery_root,
    )
}

fn generate_secret_inline() -> String {
    use std::fs;
    use std::io::Read;
    let mut buf = [0u8; 16];
    if let Ok(mut f) = fs::File::open("/dev/urandom") {
        let _ = f.read_exact(&mut buf);
    }
    buf.iter().map(|b| format!("{:02x}", b)).collect()
}
