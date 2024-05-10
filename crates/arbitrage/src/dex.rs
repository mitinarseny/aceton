use std::sync::Arc;

use fraction::Decimal;
use impl_tools::autoimpl;
use num_bigint::BigUint;
use num_traits::ToPrimitive;

use crate::Asset;

#[autoimpl(for<T: trait> &T, &mut T, Box<T>, Arc<T>)]
pub trait DexPool {
    fn assets(&self) -> [Asset; 2];
    /// In the same order as in [`.assets()`](DEXPool::assets)
    fn reserves(&self) -> [&BigUint; 2];
    /// Returns fees for the incoming and outcoming assets respectively,
    /// so that [0.002, 0] means 0.2% fees for the incoming asset and
    /// 0% fees on the outcoming asset
    fn trade_fees(&self) -> [Decimal; 2];

    fn reversed(&self, asset_in: Asset) -> bool {
        asset_in == self.assets()[1]
    }

    fn asset_out(&self, asset_in: Asset) -> Asset {
        let mut assets = self.assets();
        if self.reversed(asset_in) {
            assets.reverse();
        }
        assets[1]
    }

    fn reserves_in_out(&self, asset_in: Asset) -> [&BigUint; 2] {
        let mut reserves = self.reserves();
        if self.reversed(asset_in) {
            reserves.reverse();
        }
        reserves
    }

    fn rate(&self, asset_in: Asset) -> f64 {
        let reserves = self.reserves_in_out(asset_in).map(|r| r.to_f64().unwrap());
        reserves[1] / reserves[0]
    }
    fn rate_with_fees(&self, asset_in: Asset) -> f64 {
        self.rate(asset_in)
            * self
                .trade_fees()
                .into_iter()
                .product::<Decimal>()
                .to_f64()
                .unwrap()
    }

    fn estimate_swap_out(&self, asset_in: Asset, amount_in: BigUint) -> BigUint {
        let [reserve0, reserve1] = self.reserves_in_out(asset_in);
        let trade_fees = self.trade_fees().map(|fee| -fee + 1);
        let amount_in: BigUint = (trade_fees[0] * amount_in).trunc().try_into().unwrap();
        let amount_out = reserve1 * &amount_in / (reserve0 + amount_in);
        (trade_fees[1] * amount_out).try_into().unwrap()
    }
}
