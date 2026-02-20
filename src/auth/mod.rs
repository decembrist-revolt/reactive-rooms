use std::fmt;
use std::sync::Arc;

use axum_keycloak_auth::instance::{KeycloakAuthInstance, KeycloakConfig};
use axum_keycloak_auth::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Admin,
    Host,
    User,
    Unknown(String),
}

impl axum_keycloak_auth::role::Role for Role {}

impl From<String> for Role {
    fn from(value: String) -> Self {
        match value.as_str() {
            "reactive-rooms:scope:write" => Role::Admin,
            "reactive-rooms:scope:host" => Role::Host,
            "reactive-rooms:scope:user" => Role::User,
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

pub fn create_keycloak_instance() -> Arc<KeycloakAuthInstance> {
    let server = std::env::var("KEYCLOAK_SERVER").expect("KEYCLOAK_SERVER must be set");
    let realm = std::env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set");

    Arc::new(KeycloakAuthInstance::new(
        KeycloakConfig::builder()
            .server(Url::parse(&server).expect("Invalid KEYCLOAK_SERVER URL"))
            .realm(realm)
            .build(),
    ))
}
