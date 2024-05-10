use core::{
    fmt::{self, Display},
    iter,
};

use fraction::BigUint;
use itertools::unfold;

use crate::{Asset, DexPool};

#[derive(Debug, Clone)]
pub struct SwapStep<DP> {
    asset_in: Asset,
    pool: DP,
}

impl<DP> SwapStep<DP>
where
    DP: DexPool,
{
    pub fn asset_in(&self) -> Asset {
        self.asset_in
    }

    pub fn pool(&self) -> &DP {
        &self.pool
    }

    pub fn asset_out(&self) -> Asset {
        self.pool.asset_out(self.asset_in)
    }

    pub fn estimate_swap_out(&self, amount_in: BigUint) -> BigUint {
        self.pool.estimate_swap_out(self.asset_in, amount_in)
    }
}

impl<DP> Display for SwapStep<DP>
where
    DP: DexPool,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [r_in, r_out] = self.pool().reserves_in_out(self.asset_in);
        write!(
            f,
            "{} -[{}/{}]-> {}",
            self.asset_in(),
            r_in,
            r_out,
            self.asset_out()
        )
    }
}

#[derive(Debug, Clone)]
pub struct SwapPath<DP> {
    asset_in: Asset,
    pools: Vec<DP>,
}

impl<DP> SwapPath<DP>
where
    DP: DexPool,
{
    pub const fn new(asset_in: Asset) -> Self {
        Self {
            asset_in,
            pools: Vec::new(),
        }
    }

    pub fn asset_in(&self) -> Asset {
        self.asset_in
    }

    pub fn len(&self) -> usize {
        self.pools.len()
    }

    pub fn push(&mut self, next: DP) -> Asset {
        let asset_in = self.asset_out();
        let asset_out = next.asset_out(asset_in);
        self.pools.push(next);
        asset_out
    }

    pub fn iter_steps(&self) -> impl Iterator<Item = SwapStep<&'_ DP>> {
        unfold((self.asset_in, self.pools.iter()), |(asset_in, pools)| {
            let step = SwapStep {
                asset_in: *asset_in,
                pool: pools.next()?,
            };
            *asset_in = step.asset_out();
            Some(step)
        })
    }

    pub fn iter_assets(&self) -> impl Iterator<Item = Asset> + '_ {
        iter::once(self.asset_in).chain(self.iter_steps().map(|step| step.asset_out()))
    }

    pub fn iter_pools(&self) -> impl Iterator<Item = &DP> + '_ {
        self.iter_steps().map(|step| *step.pool())
    }

    pub fn asset_out(&self) -> Asset {
        self.iter_steps()
            .last()
            .map(|step| step.asset_out())
            .unwrap_or(self.asset_in)
    }

    pub fn is_cycle(&self) -> bool {
        self.asset_out() == self.asset_in
    }

    pub fn estimate_swap_out(&self, amount_in: BigUint) -> BigUint {
        self.iter_steps().fold(amount_in, |amount_in, step| {
            step.estimate_swap_out(amount_in)
        })
    }

    pub fn optimal_amount_in(&self) -> BigUint {
        todo!()
    }
}

impl<DP> Extend<DP> for SwapPath<DP> {
    fn extend<T: IntoIterator<Item = DP>>(&mut self, pools: T) {
        self.pools.extend(pools)
    }
}

impl<DP> Display for SwapPath<DP>
where
    DP: DexPool,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.asset_in)?;
        for step in self.iter_steps() {
            let [r_in, r_out] = step.pool().reserves_in_out(step.asset_in);
            write!(f, " -[{}/{}]-> {:?}", r_in, r_out, step.asset_out())?;
        }
        Ok(())
    }
}
