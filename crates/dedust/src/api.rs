use aceton_arbitrage::Asset;
use chrono::{DateTime, Utc};
use num::BigUint;
use reqwest::Client;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use tlb_ton::MsgAddress;
use url::Url;

use crate::DedustPool;

const BASE_URL: &str = "https://api.dedust.io/v2";

#[derive(Default)]
pub struct DedustHTTPClient {
    client: Client,
}

impl DedustHTTPClient {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn get_available_pools(&self) -> anyhow::Result<Vec<DedustPool>> {
        self.client
            .get(format!("{BASE_URL}/pools"))
            .send()
            .await?
            .json()
            .await
            .map_err(Into::into)
    }

    pub async fn get_latest_trades(
        &self,
        pool: MsgAddress,
        limit: impl Into<Option<usize>>,
    ) -> anyhow::Result<Vec<Trade>> {
        self.client
            .get(
                Url::parse_with_params(
                    format!("{BASE_URL}/pools/{pool}/trades").as_str(),
                    limit.into().map(|limit| ("page_size", limit.to_string())),
                )
                .unwrap(),
            )
            .send()
            .await?
            .json()
            .await
            .map_err(Into::into)
    }
}

#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    pub sender: MsgAddress,
    pub asset_in: Asset,
    pub asset_out: Asset,
    #[serde_as(as = "DisplayFromStr")]
    pub amount_in: BigUint,
    #[serde_as(as = "DisplayFromStr")]
    pub amount_out: BigUint,
    #[serde_as(as = "DisplayFromStr")]
    pub lt: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub created_at: DateTime<Utc>,
}
