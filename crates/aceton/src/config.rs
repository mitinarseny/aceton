use std::time::Duration;

use aceton_arbitrage::ArbitragerConfig;
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DefaultOnNull};
use tonlibjson_client::ton::TonClientBuilder;
use url::Url;

#[serde_as]
#[derive(Deserialize)]
pub struct AcetonConfig {
    #[serde_as(as = "DefaultOnNull")]
    pub ton: TonConfig,
    pub arbitrage: ArbitragerConfig,
}

#[derive(Serialize, Deserialize)]
pub struct TonConfig {
    pub config: Url,
}

impl Default for TonConfig {
    fn default() -> Self {
        Self {
            config: "https://ton.org/global-config.json".parse().unwrap(),
        }
    }
}

impl TonConfig {
    pub fn config(&self) -> anyhow::Result<TonClientBuilder> {
        Ok(match self.config.scheme() {
            "http" | "https" => {
                TonClientBuilder::from_config_url(self.config.clone(), Duration::from_secs(60))
            }
            "file" => TonClientBuilder::from_config_path(
                self.config
                    .to_file_path()
                    .ok()
                    .context("invalid file path")?,
            ),
            _ => return Err(anyhow!("invalid TON config URL: {}", self.config)),
        }
        .set_timeout(Duration::from_secs(30))
        .set_retry_percent(3.0))
    }
}
