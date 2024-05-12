use std::sync::Arc;

use async_trait::async_trait;
use impl_tools::autoimpl;
use num::BigUint;
use tlb_ton::Message;

use crate::{Asset, DexPool};

#[async_trait]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait Dex {
    type Pool: DexPool;

    async fn get_pools(&self) -> anyhow::Result<Vec<Self::Pool>>;

    async fn update_pool(&self, pool: &mut Self::Pool) -> anyhow::Result<()>;

    async fn make_message(
        &self,
        query_id: u64,
        asset_in: Asset,
        amount_in: BigUint,
        steps: <Self::Pool as DexPool>::Step,
    ) -> anyhow::Result<Message>;
}
