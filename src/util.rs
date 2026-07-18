use aws_config::BehaviorVersion;
use aws_sdk_secretsmanager::{Client, config::Region};
use fractic_server_error::ServerError;
use std::collections::HashMap;

use crate::errors::{SecretsKeyNotFound, SecretsManagerCalloutError, SecretsParsingError};

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
    /// The AWS secret is expected to be a JSON object that maps secret keys
    /// (`&str`) to their String values.
    pub async fn load_secrets(
        &self,
        secrets_id: &str,
        keys: &[&'static str],
    ) -> Result<HashMap<&'static str, String>, ServerError> {
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

        let secrets_json: HashMap<String, serde_json::Value> = serde_json::from_str(secret_string)
            .map_err(|e| {
                SecretsParsingError::with_debug("invalid secret JSON", secrets_id, &self.region, &e)
            })?;

        // Extract the requested subset.
        let mut subset = HashMap::new();
        for key in keys {
            let value = secrets_json
                .get(*key)
                .ok_or_else(|| SecretsKeyNotFound::new(key, secrets_id))?;
            subset.insert(
                *key,
                stringify_secret_value(value, secrets_id, &self.region)?,
            );
        }

        Ok(subset)
    }
}

fn stringify_secret_value(
    value: &serde_json::Value,
    secrets_id: &str,
    region: &str,
) -> Result<String, ServerError> {
    match value {
        serde_json::Value::String(value) => Ok(value.clone()),
        value => serde_json::to_string(value).map_err(|e| {
            SecretsParsingError::with_debug(
                "secret value could not be serialized",
                secrets_id,
                region,
                &e,
            )
        }),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn stringify_secret_value_preserves_strings() {
        assert_eq!(
            stringify_secret_value(&json!("plain"), "secret", "region").unwrap(),
            "plain"
        );
    }

    #[test]
    fn stringify_secret_value_serializes_structured_values() {
        assert_eq!(
            stringify_secret_value(
                &json!({"active": "new", "keys": {"new": "abc"}}),
                "secret",
                "region"
            )
            .unwrap(),
            r#"{"active":"new","keys":{"new":"abc"}}"#
        );
    }
}
