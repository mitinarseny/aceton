use serde::{Deserialize, Serialize};

use crate::Asset;

#[derive(Serialize, Deserialize)]
pub struct ArbitragerConfig {
    pub base_asset: Asset,
    pub max_length: Option<usize>,
}
