use num::{rational::Ratio, BigUint};
use serde::Deserialize;

use crate::Asset;

#[derive(Deserialize)]
pub struct ArbitragerConfig {
    pub base_asset: Asset,
    pub max_length: Option<usize>,
    // #[serde_as(as = "DecimalFloatStrAsRatio")]
    // pub amount_in_balance_coef: Ratio<BigUint>,
}

impl ArbitragerConfig {
    pub fn amount_in_balance_coef(&self) -> Ratio<BigUint> {
        Ratio::new(7u32.into(), 10u32.into())
    }
}
