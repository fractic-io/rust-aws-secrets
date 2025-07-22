use aws_sdk_secretsmanager::Client;
use std::collections::HashMap;
use aws_config::BehaviorVersion;
use aws_sdk_secretsmanager::{config::Region, Client};
use fractic_server_error::{CriticalError, ServerError};

pub struct SecretsUtil {
    client: Client,
    region: String,
}

impl SecretsUtil {
    pub async fn new(region_str: String) -> Self {
        let region = Region::new(region_str);
        let shared_config = aws_config::defaults(BehaviorVersion::v2025_01_17())
            .region(region)
            .load()
            .await;
        let client = Client::new(&shared_config);
        Self { client, region: region_str }
    }

    /// Fetch a subset of secrets from AWS Secrets Manager.
    ///
    /// The AWS secret is expected to be a JSON object that maps secret keys
    /// (`&str`) to their String values.
    ///
    /// The function will retrieve the JSON text in a single network round-trip,
    /// parse it once and then return a map that contains the requested subset of
    /// keys.  If a requested key is missing, or the JSON is malformed, a
    /// `ServerError` will be raised to fail fast – this mirrors the behaviour of
    /// the `Env` helpers elsewhere in the crate, where the application must not
    /// start with an invalid configuration.
    pub async fn load_secrets(
        &self,
        secrets_id: &str,
        keys: &[&'static str],
    ) -> Result<HashMap<&'static str, String>, ServerError> {
        // Retrieve the JSON blob from Secrets Manager.
        let secrets_output = self.client
            .get_secret_value()
            .secret_id(&secrets_id)
            .send()
            .await
            .map_err(|e| {
                SecretsManagerCalloutError::with_debug(
                    "failed to fetch secrets JSON",
                    secrets_id, self.region, &e
                )
            })?;

        let secret_string = secrets_output.secret_string().ok_or_else(|| {
            SecretsParsingError::new(
                "secret value is empty or binary",
                secrets_id, self.region
            )
        })?;

        let secrets_json: HashMap<String, String> = serde_json::from_str(secret_string).map_err(|e| {
            SecretsParsingError::with_debug(
                "invalid secret JSON",
                secrets_id, self.region, &e
            )
        })?;

        // Extract the requested subset.
        let mut subset = HashMap::new();
        for key in keys {
            let value = secrets_json.get(*key).ok_or_else(|| {
                SecretsKeyNotFound::new(key, secrets_id)
            })?;
            subset.insert(*key, value.clone());
        }

        Ok(subset)
    }
}