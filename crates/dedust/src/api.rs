use reqwest::Client;
use serde::Deserialize;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use tlb_ton::MsgAddress;

use crate::{DedustAsset, DedustPoolType};

const BASE_URL: &str = "https://api.dedust.io/v2";

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
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DedustPool {
    pub address: MsgAddress,
    #[serde_as(as = "DisplayFromStr")]
    pub r#type: DedustPoolType,
    #[serde_as(as = "DisplayFromStr")]
    pub trade_fee: f64,
    pub assets: [DedustAsset; 2],
}
