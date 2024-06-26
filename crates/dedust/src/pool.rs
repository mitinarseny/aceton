use aceton_core::{
    ton_utils::{adapters::TvmBoxedStackEntryExt, contract::TonContractI},
    Asset, AssetWithMetadata, DexPool,
};
use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use impl_tools::autoimpl;
use num::{rational::Ratio, traits::ConstZero, BigUint, ToPrimitive};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use strum::EnumString;
use tlb::{
    BitPack, BitReader, BitReaderExt, BitUnpack, BitWriter, BitWriterExt, CellBuilder,
    CellBuilderError, CellDeserialize, CellParser, CellParserError, CellSerialize, Data, Ref,
};
use tlb_ton::{Coins, MsgAddress, UnixTimestamp};

use aceton_utils::DecimalFloatStrAsRatio;

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
    /// Specifies a deadline for the swap.
    /// If the swap reaches the Pool after this time, it will be rejected.  
    /// **Default**: 0 (disabled).
    pub deadline: Option<DateTime<Utc>>,
    /// Specifies an address where funds will be sent after the swap.  
    /// **Default**: sender's address.
    pub recepient: MsgAddress,
    /// Referral address. Required for the Referral Program.
    pub referral: MsgAddress,
    /// Custom payload that will be attached to the fund transfer upon a **successful** swap.
    pub fulfill_payload: Option<F>,
    /// Custom payload that will be attached to the fund transfer upon a **rejected** swap.
    pub reject_payload: Option<R>,
}

impl<F, R> CellSerialize for SwapParams<F, R>
where
    F: CellSerialize,
    R: CellSerialize,
{
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder
            .pack_as::<_, UnixTimestamp>(self.deadline.unwrap_or(DateTime::UNIX_EPOCH))?
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
            deadline: Some(parser.unpack_as::<_, UnixTimestamp>()?)
                .filter(|timestamp| *timestamp == DateTime::UNIX_EPOCH),
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

impl SwapStep {
    pub fn len(&self) -> usize {
        // TODO: without recursion
        1 + self.params.next.as_deref().map_or(0, SwapStep::len)
    }
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
    #[serde_as(as = "DecimalFloatStrAsRatio")]
    pub trade_fee: Ratio<BigUint>,
    #[serde_as(as = "[DisplayFromStr; 2]")]
    pub reserves: [BigUint; 2],
}

impl DexPool for DedustPool {
    type ID = MsgAddress;
    type Step = SwapStep;

    #[inline]
    fn id(&self) -> Self::ID {
        self.address
    }

    #[inline]
    fn assets(&self) -> [Asset; 2] {
        let (a0, a1) = (self.assets[0].asset, self.assets[1].asset);
        [a0, a1].map(Into::into)
    }

    #[inline]
    fn reserves(&self) -> [&BigUint; 2] {
        let [ref r0, ref r1] = &self.reserves;
        [r0, r1]
    }

    // TODO: pool type
    #[inline]
    fn trade_fees(&self) -> [Ratio<BigUint>; 2] {
        [
            Ratio::from_integer(BigUint::from(1u32)) - &self.trade_fee / BigUint::from(100u32),
            Ratio::from_integer(1u32.into()),
        ]
    }

    #[inline]
    fn ratio(&self, asset_in: Asset) -> Ratio<BigUint> {
        match self.r#type {
            DedustPoolType::Volatile => {
                let [r_in, r_out] = self.reserves_in_out(asset_in);
                Ratio::new(r_out.clone(), r_in.clone())
            }
            DedustPoolType::Stable => Ratio::from_integer(1u32.into()),
        }
    }

    #[inline]
    fn rate(&self, asset_in: Asset) -> f64 {
        self.ratio(asset_in).to_f64().unwrap()
    }

    #[inline]
    fn rate_with_fees(&self, asset_in: Asset) -> f64 {
        (self.ratio(asset_in) * self.trade_fees().into_iter().product::<Ratio<BigUint>>())
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
        match self.r#type {
            DedustPoolType::Volatile => {
                let amount_out = (&amount_in_with_fee * reserve_out
                    / (amount_in_with_fee + reserve_in))
                    .to_integer();
                if &amount_out >= reserve_out {
                    return BigUint::ZERO;
                }
                (fee_out * amount_out).to_integer()
            }
            // TODO: real stable swap formula
            DedustPoolType::Stable => {
                (amount_in_with_fee * Ratio::new(reserve_out.clone(), reserve_in.clone())).to_integer()
            }
        }
    }

    fn make_step(&self, amount_out_min: Option<BigUint>, next: Option<Self::Step>) -> Self::Step {
        SwapStep {
            pool: self.address,
            params: SwapStepParams {
                kind: SwapKind::GivenIn,
                limit: amount_out_min.unwrap_or(BigUint::ZERO),
                next: next.map(Box::new),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_step_len() {
        assert_eq!(
            SwapStep {
                pool: MsgAddress::NULL,
                params: SwapStepParams {
                    kind: SwapKind::GivenIn,
                    limit: BigUint::ZERO,
                    next: Some(
                        SwapStep {
                            pool: MsgAddress::NULL,
                            params: SwapStepParams {
                                kind: SwapKind::GivenIn,
                                limit: BigUint::ZERO,
                                next: Some(
                                    SwapStep {
                                        pool: MsgAddress::NULL,
                                        params: SwapStepParams {
                                            kind: SwapKind::GivenIn,
                                            limit: BigUint::ZERO,
                                            next: None,
                                        },
                                    }
                                    .into()
                                ),
                            },
                        }
                        .into()
                    ),
                },
            }
            .len(),
            3
        )
    }
}
