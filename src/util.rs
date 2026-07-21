use aws_config::BehaviorVersion;
use aws_sdk_secretsmanager::{Client, config::Region};
use fractic_server_error::ServerError;
use serde_json::Value;
use std::collections::HashMap;

use crate::errors::{SecretsKeyNotFound, SecretsManagerCalloutError, SecretsParsingError};

// Public interface.
// ----------------------------------------------------------------------------

pub struct SecretsUtil {
    client: Client,
    region: String,
}

impl SecretsUtil {
    pub async fn new(region: String) -> Self {
        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region.clone()))
            .load()
            .await;
        let client = Client::new(&shared_config);
        Self { client, region }
    }

    /// Fetch a subset of secrets from AWS Secrets Manager.
    ///
    /// The AWS secret must be a JSON object. Selected values are returned
    /// without changing their JSON types.
    pub async fn load_secrets(
        &self,
        secrets_id: &str,
        keys: &[&str],
    ) -> Result<HashMap<String, Value>, ServerError> {
        // Retrieve the JSON blob from Secrets Manager.
        let secrets_output = self
            .client
            .get_secret_value()
            .secret_id(secrets_id)
            .send()
            .await
            .map_err(|e| {
                SecretsManagerCalloutError::with_debug(
                    "failed to fetch secrets JSON",
                    secrets_id,
                    &self.region,
                    &e,
                )
            })?;

        let secret_string = secrets_output.secret_string().ok_or_else(|| {
            SecretsParsingError::new("secret value is empty or binary", secrets_id, &self.region)
        })?;

        parse_secret_values(secret_string, secrets_id, &self.region, keys)
    }
}

// Internal.
// ----------------------------------------------------------------------------

fn parse_secret_values(
    secret_string: &str,
    secrets_id: &str,
    region: &str,
    keys: &[&str],
) -> Result<HashMap<String, Value>, ServerError> {
    let mut secrets_json: HashMap<String, Value> =
        serde_json::from_str(secret_string).map_err(|e| {
            SecretsParsingError::with_debug("invalid secret JSON", secrets_id, region, &e)
        })?;

    keys.iter()
        .map(|&key| {
            secrets_json
                .remove(key)
                .map(|value| (key.to_owned(), value))
                .ok_or_else(|| SecretsKeyNotFound::new(key, secrets_id))
        })
        .collect()
}

// Tests.
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_secret_values_preserves_json_types() {
        let values = parse_secret_values(
            r#"{"plain":"value","structured":{"active":"new"}}"#,
            "secret",
            "region",
            &["plain", "structured"],
        )
        .unwrap();

        assert_eq!(values.get("plain"), Some(&json!("value")));
        assert_eq!(values.get("structured"), Some(&json!({"active": "new"})));
    }
}
