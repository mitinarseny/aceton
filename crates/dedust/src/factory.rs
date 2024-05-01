use aceton_core::{TonContractI, TvmBoxedStackEntryExt};
use anyhow::anyhow;
use async_trait::async_trait;
use hex_literal::hex;
use tlb::Data;
use tlb_ton::MsgAddress;

use crate::{DedustAsset, DedustPoolType};

pub const DEDUST_FACTORY_ADDRESS: MsgAddress = MsgAddress {
    workchain_id: 0,
    address: hex!("5f0564fb5f604783db57031ce1cf668a88d4d4d6da6de4db222b4b920d6fd800"),
};

#[async_trait]
pub trait DedustFactoryI: TonContractI {
    async fn get_vault_address(&self, asset: DedustAsset) -> anyhow::Result<MsgAddress> {
        let [asset] = self
            .get(
                "get_vault_address",
                [TvmBoxedStackEntryExt::store_cell_as::<_, Data>(asset)?].into(),
            )
            .await??
            .try_into()
            .map_err(|stack| anyhow!("invalid stack: {stack:?}"))?;
        asset.parse_cell_fully_as::<_, Data>()
    }

    async fn get_pool_address(
        &self,
        r#type: DedustPoolType,
        assets: [DedustAsset; 2],
    ) -> anyhow::Result<MsgAddress> {
        let [pool] = self
            .get(
                "get_vault_address",
                [
                    TvmBoxedStackEntryExt::from_number(r#type as u8),
                    TvmBoxedStackEntryExt::store_cell_as::<_, Data>(assets[0])?,
                    TvmBoxedStackEntryExt::store_cell_as::<_, Data>(assets[1])?,
                ]
                .into(),
            )
            .await??
            .try_into()
            .map_err(|stack| anyhow!("invalid stack: {stack:?}"))?;
        pool.parse_cell_fully_as::<_, Data>()
    }

    async fn get_liquidity_deposit_address(
        &self,
        owner: MsgAddress,
        r#type: DedustPoolType,
        assets: [DedustAsset; 2],
    ) -> anyhow::Result<MsgAddress> {
        let [liquidity_deposit_addr] = self
            .get(
                "get_vault_address",
                [
                    TvmBoxedStackEntryExt::store_cell_as::<_, Data>(owner)?,
                    TvmBoxedStackEntryExt::from_number(r#type as u8),
                    TvmBoxedStackEntryExt::store_cell_as::<_, Data>(assets[0])?,
                    TvmBoxedStackEntryExt::store_cell_as::<_, Data>(assets[1])?,
                ]
                .into(),
            )
            .await??
            .try_into()
            .map_err(|stack| anyhow!("invalid stack: {stack:?}"))?;
        liquidity_deposit_addr.parse_cell_fully_as::<_, Data>()
    }
}

impl<C> DedustFactoryI for C where C: TonContractI {}
