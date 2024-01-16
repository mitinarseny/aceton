use aceton::AppConfig;

use serde::Deserialize;
use tonlib::client::TonClient;
use tracing::instrument;

#[derive(Deserialize)]
pub struct AcetonConfig {
    #[serde(flatten)]
    pub config: AppConfig,
}

impl AcetonConfig {
    #[instrument(skip_all)]
    pub async fn make_ton_client(&self) -> anyhow::Result<TonClient> {
        let client = reqwest::Client::new();
        let config = client
            .get("https://ton.org/global-config.json")
            .send()
            .await?
            .text()
            .await?;

        TonClient::set_log_verbosity_level(0);
        TonClient::builder()
            .with_config(&config)
            .build()
            .await
            .map_err(Into::into)
    }
}
