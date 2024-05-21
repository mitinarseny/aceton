use anyhow::anyhow;
use async_trait::async_trait;

use crate::{adapters::TvmBoxedStackEntryExt, contract::TonContractI};

#[async_trait]
pub trait WalletI: TonContractI {
    async fn seqno(&self) -> anyhow::Result<u32> {
        let [seqno] = self
            .get("seqno", [].into())
            .await??
            .try_into()
            .map_err(|stack| anyhow!("invalid output stack size: {stack:?}"))?;
        seqno.into_number()
    }
}

impl<C> WalletI for C where C: TonContractI {}
