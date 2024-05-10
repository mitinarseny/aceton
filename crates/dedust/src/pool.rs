use anyhow::anyhow;
use async_trait::async_trait;
use fraction::Decimal;
use impl_tools::autoimpl;
use num_bigint::BigUint;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use strum::EnumString;
use tlb::{
    BitPack, BitReader, BitReaderExt, BitUnpack, BitWriter, BitWriterExt, CellBuilder,
    CellBuilderError, CellDeserialize, CellParser, CellParserError, CellSerialize, Data, Ref,
};
use tlb_ton::{Coins, MsgAddress};

use aceton_arbitrage::{Asset, AssetWithMetadata, DexPool};
use aceton_core::{TonContractI, TvmBoxedStackEntryExt};

use crate::DedustAsset;

#[async_trait]
pub trait DedustPoolI: TonContractI {
    async fn get_assets(&self) -> anyhow::Result<[Asset; 2]> {
        let [asset0, asset1] = self
            .get("get_assets", [].into())
            .await??
            .try_into()
            .map_err(|stack| anyhow!("invalid output stack size: {stack:?}"))?;

        let asset0 = asset0.parse_cell_fully_as::<_, Data<DedustAsset>>()?;
        let asset1 = asset1.parse_cell_fully_as::<_, Data<DedustAsset>>()?;

        Ok([asset0, asset1])
    }

    async fn get_reserves(&self) -> anyhow::Result<[BigUint; 2]> {
        let [reserve0, reserve1] = self
            .get("get_reserves", [].into())
            .await??
            .try_into()
            .map_err(|stack| anyhow!("invalid output stack: {stack:?}"))?;

        let reserve0 = reserve0.into_number()?;
        let reserve1 = reserve1.into_number()?;

        Ok([reserve0, reserve1])
    }

    async fn is_stable(&self) -> anyhow::Result<bool> {
        let [is_stable] = self
            .get("is_stable", [].into())
            .await??
            .try_into()
            .map_err(|stack| anyhow!("invalid output stack: {stack:?}"))?;

        Ok(is_stable.into_number::<u8>()? == 1)
    }

    async fn estimate_swap_out(
        &self,
        asset_in: Asset,
        amount_in: BigUint,
    ) -> anyhow::Result<EstimateSwapOutResult> {
        let [asset_out, amount_out, trade_fee] = self
            .get(
                "estimate_swap_out",
                [
                    TvmBoxedStackEntryExt::store_cell_as::<_, Data<DedustAsset>>(asset_in)?,
                    TvmBoxedStackEntryExt::from_number(amount_in),
                ]
                .into(),
            )
            .await??
            .try_into()
            .map_err(|stack| anyhow!("invalid output stack: {stack:?}"))?;
        Ok(EstimateSwapOutResult {
            asset_out: asset_out.parse_cell_fully_as::<_, Data<DedustAsset>>()?,
            amount_out: amount_out.into_number()?,
            trade_fee: trade_fee.into_number()?,
        })
    }
}

impl<C> DedustPoolI for C where C: TonContractI {}

pub struct EstimateSwapOutResult {
    pub asset_out: Asset,
    /// amount of asset_out
    pub amount_out: BigUint,
    /// amount of asset_in asset given as a fee
    pub trade_fee: BigUint,
}

#[derive(Clone, Copy)]
pub enum SwapKind {
    // given_in$0 = SwapKind;
    GivenIn,
    // given_out$1 = SwapKind; // Not implemented.
    GivenOut,
}

impl BitPack for SwapKind {
    fn pack<W>(&self, writer: W) -> Result<(), W::Error>
    where
        W: BitWriter,
    {
        match self {
            Self::GivenIn => false,
            Self::GivenOut => true,
        }
        .pack(writer)
    }
}

impl BitUnpack for SwapKind {
    fn unpack<R>(mut reader: R) -> Result<Self, R::Error>
    where
        R: BitReader,
    {
        Ok(match reader.unpack()? {
            false => Self::GivenIn,
            true => Self::GivenOut,
        })
    }
}

pub type Timestamp = u32;

/// swap_params#_ deadline:Timestamp recipient_addr:MsgAddressInt referral_addr:MsgAddress
/// fulfill_payload:(Maybe ^Cell) reject_payload:(Maybe ^Cell) = SwapParams;
pub struct SwapParams<F, R> {
    pub deadline: Timestamp,
    pub recepient: MsgAddress,
    pub referral: MsgAddress,
    pub fulfill_payload: Option<F>,
    pub reject_payload: Option<R>,
}

impl<F, R> CellSerialize for SwapParams<F, R>
where
    F: CellSerialize,
    R: CellSerialize,
{
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder
            .pack(self.deadline)?
            .pack(self.recepient)?
            .pack(self.referral)?
            .store_as::<_, Option<Ref>>(self.fulfill_payload.as_ref())?
            .store_as::<_, Option<Ref>>(self.reject_payload.as_ref())?;
        Ok(())
    }
}

impl<'de, F, R> CellDeserialize<'de> for SwapParams<F, R>
where
    F: CellDeserialize<'de>,
    R: CellDeserialize<'de>,
{
    fn parse(parser: &mut CellParser<'de>) -> Result<Self, CellParserError<'de>> {
        Ok(Self {
            deadline: parser.unpack()?,
            recepient: parser.unpack()?,
            referral: parser.unpack()?,
            fulfill_payload: parser.parse_as::<_, Option<Ref>>()?,
            reject_payload: parser.parse_as::<_, Option<Ref>>()?,
        })
    }
}

/// step#_ pool_addr:MsgAddressInt params:SwapStepParams = SwapStep;
pub struct SwapStep {
    pub pool: MsgAddress,
    pub params: SwapStepParams,
}

impl CellSerialize for SwapStep {
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder.pack(self.pool)?.store(&self.params)?;
        Ok(())
    }
}

impl<'de> CellDeserialize<'de> for SwapStep {
    fn parse(parser: &mut CellParser<'de>) -> Result<Self, CellParserError<'de>> {
        Ok(Self {
            pool: parser.unpack()?,
            params: parser.parse()?,
        })
    }
}

/// step_params#_ kind:SwapKind limit:Coins next:(Maybe ^SwapStep) = SwapStepParams;
pub struct SwapStepParams {
    pub kind: SwapKind,
    pub limit: BigUint,
    pub next: Option<Box<SwapStep>>,
}

impl CellSerialize for SwapStepParams {
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder
            .pack(self.kind)?
            .pack_as::<_, &Coins>(&self.limit)?
            .store_as::<_, Option<Ref>>(self.next.as_ref())?;
        Ok(())
    }
}

impl<'de> CellDeserialize<'de> for SwapStepParams {
    fn parse(parser: &mut CellParser<'de>) -> Result<Self, CellParserError<'de>> {
        Ok(Self {
            kind: parser.unpack()?,
            limit: parser.unpack_as::<_, Coins>()?,
            next: parser.parse_as::<_, Option<Ref>>()?,
        })
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum DedustPoolType {
    /// volatile$0 = PoolType;
    Volatile,
    /// stable$1 = PoolType;
    Stable,
}

impl BitPack for DedustPoolType {
    fn pack<W>(&self, writer: W) -> Result<(), W::Error>
    where
        W: BitWriter,
    {
        match self {
            Self::Volatile => false,
            Self::Stable => true,
        }
        .pack(writer)
    }
}

impl BitUnpack for DedustPoolType {
    fn unpack<R>(mut reader: R) -> Result<Self, R::Error>
    where
        R: BitReader,
    {
        Ok(match reader.unpack()? {
            false => Self::Volatile,
            true => Self::Stable,
        })
    }
}

/// pool_params#_ pool_type:PoolType asset0:Asset asset1:Asset = PoolParams;
pub struct PoolParams {
    pub r#type: DedustPoolType,
    pub assets: [Asset; 2],
}

impl BitPack for PoolParams {
    fn pack<W>(&self, mut writer: W) -> Result<(), W::Error>
    where
        W: BitWriter,
    {
        writer
            .pack(self.r#type)?
            .pack_many_as::<_, &DedustAsset>(&self.assets)?;
        Ok(())
    }
}

impl BitUnpack for PoolParams {
    fn unpack<R>(mut reader: R) -> Result<Self, R::Error>
    where
        R: BitReader,
    {
        Ok(Self {
            r#type: reader.unpack()?,
            assets: reader.unpack_as::<_, [DedustAsset; 2]>()?,
        })
    }
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[autoimpl(PartialEq ignore self.r#type, self.trade_fee, self.assets)]
#[autoimpl(Eq)]
#[autoimpl(Hash ignore self.r#type, self.trade_fee, self.assets)]
#[serde(rename_all = "camelCase")]
pub struct DedustPool {
    pub address: MsgAddress,
    #[serde_as(as = "DisplayFromStr")]
    pub r#type: DedustPoolType,
    pub assets: [AssetWithMetadata; 2],
    #[serde_as(as = "DisplayFromStr")]
    pub trade_fee: Decimal,
    #[serde_as(as = "[DisplayFromStr; 2]")]
    pub reserves: [BigUint; 2],
}

impl DexPool for DedustPool {
    fn assets(&self) -> [Asset; 2] {
        let (a0, a1) = (self.assets[0].asset, self.assets[1].asset);
        [a0, a1].map(Into::into)
    }

    fn reserves(&self) -> [&BigUint; 2] {
        let [ref r0, ref r1] = &self.reserves;
        [r0, r1]
    }

    fn trade_fees(&self) -> [Decimal; 2] {
        [Decimal::from(1) - (self.trade_fee / 100), 1.into()]
    }

    // TODO: pool type
}
