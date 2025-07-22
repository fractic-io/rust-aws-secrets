use fractic_server_error::{define_client_error, define_internal_error, define_user_error};

define_internal_error!(
    SecretsManagerCalloutError,
    "Secrets Manager callout error: {details} (secret: {secrets_id}, region: {region}).",
    { details: &str, secrets_id: &str, region: &str }
);

define_internal_error!(
    SecretsParsingError,
    "Secrets parsing error: {details} (secret: {secrets_id}, region: {region}).",
    { details: &str, secrets_id: &str, region: &str }
);

define_user_error!(
    SecretsKeyNotFound,
    "Requested secret key '{missing_key}' does not exist in secret '{secrets_id}'.",
    { missing_key: &str, secrets_id: &str }
);
