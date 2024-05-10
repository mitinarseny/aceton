use aceton_arbitrage::Asset;
use aceton_core::{TonContractI, TvmBoxedStackEntryExt};
use anyhow::anyhow;
use async_trait::async_trait;
use hex_literal::hex;
use tlb::{
    BitReaderExt, BitWriterExt, CellBuilder, CellBuilderError, CellDeserialize, CellParser,
    CellParserError, CellSerialize, ConstU32, Data,
};
use tlb_ton::MsgAddress;

use crate::{DedustAsset, DedustPoolType};

pub const DEDUST_FACTORY_MAINNET_ADDRESS: MsgAddress = MsgAddress {
    workchain_id: 0,
    address: hex!("5f0564fb5f604783db57031ce1cf668a88d4d4d6da6de4db222b4b920d6fd800"),
};

#[async_trait]
pub trait DedustFactoryI: TonContractI {
    async fn get_vault_address(&self, asset: Asset) -> anyhow::Result<MsgAddress> {
        let [asset] = self
            .get(
                "get_vault_address",
                [TvmBoxedStackEntryExt::store_cell_as::<_, Data<DedustAsset>>(asset)?].into(),
            )
            .await??
            .try_into()
            .map_err(|stack| anyhow!("invalid stack: {stack:?}"))?;
        asset.parse_cell_fully_as::<_, Data>()
    }

    async fn get_pool_address(
        &self,
        r#type: DedustPoolType,
        assets: [Asset; 2],
    ) -> anyhow::Result<MsgAddress> {
        let [pool] = self
            .get(
                "get_vault_address",
                [
                    TvmBoxedStackEntryExt::from_number(r#type as u8),
                    TvmBoxedStackEntryExt::store_cell_as::<_, Data<DedustAsset>>(assets[0])?,
                    TvmBoxedStackEntryExt::store_cell_as::<_, Data<DedustAsset>>(assets[1])?,
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
        assets: [Asset; 2],
    ) -> anyhow::Result<MsgAddress> {
        let [liquidity_deposit_addr] = self
            .get(
                "get_vault_address",
                [
                    TvmBoxedStackEntryExt::store_cell_as::<_, Data>(owner)?,
                    TvmBoxedStackEntryExt::from_number(r#type as u8),
                    TvmBoxedStackEntryExt::store_cell_as::<_, Data<DedustAsset>>(assets[0])?,
                    TvmBoxedStackEntryExt::store_cell_as::<_, Data<DedustAsset>>(assets[1])?,
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

const FACTORY_CREATE_VAULT_TAG: u32 = 0x21cfe02b;

/// create_vault#21cfe02b query_id:uint64 asset:Asset = InMsgBody;
pub struct DedustFactoryCreateVault {
    pub query_id: u64,
    pub asset: Asset,
}

impl CellSerialize for DedustFactoryCreateVault {
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder
            .pack(FACTORY_CREATE_VAULT_TAG)?
            .pack(self.query_id)?
            .pack_as::<_, &DedustAsset>(&self.asset)?;
        Ok(())
    }
}

impl<'de> CellDeserialize<'de> for DedustFactoryCreateVault {
    fn parse(parser: &mut CellParser<'de>) -> Result<Self, CellParserError<'de>> {
        parser.unpack::<ConstU32<FACTORY_CREATE_VAULT_TAG>>()?;
        Ok(Self {
            query_id: parser.unpack()?,
            asset: parser.unpack_as::<_, DedustAsset>()?,
        })
    }
}

const FACTORY_CREATE_VOLATILE_POOL_TAG: u32 = 0x97d51f2f;

/// create_volatile_pool#97d51f2f query_id:uint64 asset0:Asset asset1:Asset = InMsgBody;
pub struct DedustFactoryCreateVolalitePool {
    pub query_id: u64,
    pub assets: [Asset; 2],
}

impl CellSerialize for DedustFactoryCreateVolalitePool {
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder
            .pack(FACTORY_CREATE_VOLATILE_POOL_TAG)?
            .pack(self.query_id)?
            .pack_many_as::<_, &DedustAsset>(&self.assets)?;
        Ok(())
    }
}

impl<'de> CellDeserialize<'de> for DedustFactoryCreateVolalitePool {
    fn parse(parser: &mut CellParser<'de>) -> Result<Self, CellParserError<'de>> {
        parser.unpack::<ConstU32<FACTORY_CREATE_VOLATILE_POOL_TAG>>()?;
        Ok(Self {
            query_id: parser.unpack()?,
            assets: parser.unpack_as::<_, [DedustAsset; 2]>()?,
        })
    }
}
