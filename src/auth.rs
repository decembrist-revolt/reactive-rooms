use std::sync::Arc;
use std::{fmt, sync::OnceLock};

use axum_keycloak_auth::{
    Url,
    instance::{KeycloakAuthInstance, KeycloakConfig},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Admin,
    Host,
    User,
    Unknown(String),
}

impl axum_keycloak_auth::role::Role for Role {}

static KEYCLOAK: OnceLock<Arc<KeycloakAuthInstance>> = OnceLock::new();
const ROLE_ADMIN: &str = "reactive-rooms:scope:write";
const ROLE_HOST: &str = "reactive-rooms:scope:host";
const ROLE_USER: &str = "reactive-rooms:scope:user";

impl Role {
    pub fn satisfies(&self, required: &Role) -> bool {
        *self == Role::Admin || self == required
    }
}

pub fn has_role(token: &axum_keycloak_auth::decode::KeycloakToken<Role>, required: &Role) -> bool {
    token.roles.iter().any(|r| r.role().satisfies(required))
}

impl From<String> for Role {
    fn from(value: String) -> Self {
        match value.as_str() {
            ROLE_ADMIN => Role::Admin,
            ROLE_HOST => Role::Host,
            ROLE_USER => Role::User,
            _ => Role::Unknown(value),
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Admin => f.write_str("Admin"),
            Role::Host => f.write_str("Host"),
            Role::User => f.write_str("User"),
            Role::Unknown(s) => write!(f, "Unknown: {s}"),
        }
    }
}

pub fn keycloak_audience() -> String {
    std::env::var("KEYCLOAK_AUDIENCE").unwrap_or_else(|_| "account".to_string())
}

pub fn init_keycloak() -> Result<(), String> {
    let server =
        std::env::var("KEYCLOAK_SERVER").map_err(|_| "KEYCLOAK_SERVER must be set".to_string())?;
    let realm =
        std::env::var("KEYCLOAK_REALM").map_err(|_| "KEYCLOAK_REALM must be set".to_string())?;
    let url = Url::parse(&server).map_err(|e| format!("Invalid KEYCLOAK_SERVER URL: {e}"))?;

    let instance = Arc::new(KeycloakAuthInstance::new(
        KeycloakConfig::builder().server(url).realm(realm).build(),
    ));

    KEYCLOAK
        .set(instance)
        .map_err(|_| "Keycloak already initialized".to_string())
}

pub fn keycloak() -> &'static Arc<KeycloakAuthInstance> {
    KEYCLOAK.get().expect("Keycloak not initialized")
}
