use reqwest::Client;

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
}
