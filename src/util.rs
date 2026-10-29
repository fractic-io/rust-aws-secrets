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
    /// The AWS secret must be a JSON object. A missing required key is an
    /// error; a missing optional key is omitted from the result. Selected
    /// values are returned without changing their JSON types.
    pub async fn load_secrets(
        &self,
        secrets_id: &str,
        required_keys: &[&str],
        optional_keys: &[&str],
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

        parse_secret_values(
            secret_string,
            secrets_id,
            &self.region,
            required_keys,
            optional_keys,
        )
    }
}

// Internal.
// ----------------------------------------------------------------------------

fn parse_secret_values(
    secret_string: &str,
    secrets_id: &str,
    region: &str,
    required_keys: &[&str],
    optional_keys: &[&str],
) -> Result<HashMap<String, Value>, ServerError> {
    let mut secrets_json: HashMap<String, Value> =
        serde_json::from_str(secret_string).map_err(|e| {
            SecretsParsingError::with_debug("invalid secret JSON", secrets_id, region, &e)
        })?;

    let mut selected = HashMap::new();
    for &key in required_keys {
        let value = secrets_json
            .get(key)
            .ok_or_else(|| SecretsKeyNotFound::new(key, secrets_id))?;
        selected.insert(key.to_owned(), value.clone());
    }
    for &key in optional_keys {
        if let Some(value) = secrets_json.remove(key) {
            selected.insert(key.to_owned(), value);
        }
    }
    Ok(selected)
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
            &[],
        )
        .unwrap();

        assert_eq!(values.get("plain"), Some(&json!("value")));
        assert_eq!(values.get("structured"), Some(&json!({"active": "new"})));
    }
    #[test]
    fn optional_keys_preserve_null_and_invalid_values_but_allow_absence() {
        let values = parse_secret_values(
            r#"{"required":"a","null":null,"invalid":42}"#,
            "id",
            "region",
            &["required"],
            &["missing", "null", "invalid"],
        )
        .unwrap();
        assert!(!values.contains_key("missing"));
        assert_eq!(values["null"], Value::Null);
        assert_eq!(values["invalid"], json!(42));
        assert!(serde_json::from_value::<Option<String>>(values["invalid"].clone()).is_err());
        assert!(parse_secret_values("{}", "id", "region", &["required"], &["required"]).is_err());
        assert!(parse_secret_values("broken", "id", "region", &[], &["optional"]).is_err());
    }
}
