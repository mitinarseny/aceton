use std::sync::Arc;

use impl_tools::autoimpl;
use num::{rational::Ratio, BigUint, ToPrimitive};

use crate::Asset;

#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait DexPool {
    type Step;

    fn assets(&self) -> [Asset; 2];
    /// In the same order as in [`.assets()`](DEXPool::assets)
    fn reserves(&self) -> [&BigUint; 2];
    /// Returns fees for the incoming and outcoming assets respectively,
    /// so that [0.002, 0] means 0.2% fees for the incoming asset and
    /// 0% fees on the outcoming asset
    fn trade_fees(&self) -> [Ratio<BigUint>; 2];

    fn make_step(&self, amount_out_min: Option<BigUint>, next: Option<Self::Step>) -> Self::Step;

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
        let [r_in, r_out] = self.reserves_in_out(asset_in);
        Ratio::new(r_out.clone(), r_in.clone()).to_f64().unwrap()
    }

    fn rate_with_fees(&self, asset_in: Asset) -> f64 {
        self.rate(asset_in)
            * self
                .trade_fees()
                .into_iter()
                .product::<Ratio<BigUint>>()
                .to_f64()
                .unwrap()
    }

    fn estimate_swap_out(&self, asset_in: Asset, amount_in: &BigUint) -> BigUint {
        let [reserve_in, reserve_out] = self.reserves_in_out(asset_in);
        if [amount_in, reserve_in, reserve_out]
            .into_iter()
            .any(|v| v == &BigUint::ZERO)
        {
            return BigUint::ZERO;
        }

        let [fee_in, fee_out] = self.trade_fees();
        let amount_in_with_fee = fee_in * amount_in;
        let amount_out =
            (&amount_in_with_fee * reserve_out / (amount_in_with_fee + reserve_in)).to_integer();
        if &amount_out >= reserve_out {
            return BigUint::ZERO;
        }
        (fee_out * amount_out).to_integer()
    }
}

#[cfg(test)]
mod tests {
    use tlb_ton::MsgAddress;

    use super::*;

    struct MockPool {
        assets: [Asset; 2],
        reserves: [BigUint; 2],
    }

    impl DexPool for MockPool {
        type Step = ();

        fn assets(&self) -> [Asset; 2] {
            self.assets
        }

        fn reserves(&self) -> [&BigUint; 2] {
            let [ref r0, ref r1] = &self.reserves;
            [r0, r1]
        }

        fn trade_fees(&self) -> [Ratio<BigUint>; 2] {
            [
                Ratio::new(997u32.into(), 1000u32.into()),
                Ratio::from_integer(1u32.into()),
            ]
        }

        fn make_step(
            &self,
            _amount_out_min: Option<BigUint>,
            _next: Option<Self::Step>,
        ) -> Self::Step {
            ()
        }
    }

    #[test]
    fn estimate_swap_out() {
        let p = MockPool {
            assets: [Asset::Native, Asset::Jetton(MsgAddress::NULL)],
            reserves: [10_000u32, 20_000u32].map(Into::into),
        };
        assert_eq!(
            p.estimate_swap_out(p.assets[0], &1_000u32.into()),
            1813u32.into()
        );
        assert_eq!(
            p.estimate_swap_out(p.assets[1], &1_000u32.into()),
            474u32.into()
        );
        assert_eq!(
            p.estimate_swap_out(p.assets[0], &10_000_000u32.into()),
            19979u32.into()
        );
    }
}
