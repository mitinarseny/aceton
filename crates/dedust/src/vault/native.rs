use num::BigUint;
use tlb::{
    BitReaderExt, BitWriterExt, CellBuilder, CellBuilderError, CellDeserialize, CellParserError,
    CellSerialize, ConstU32, Ref,
};
use tlb_ton::Coins;

use crate::{PoolParams, SwapParams, SwapStep};

const NATIVE_VAULT_SWAP_TAG: u32 = 0xea06185d;

/// swap#ea06185d query_id:uint64 amount:Coins _:SwapStep swap_params:^SwapParams = InMsgBody;
pub struct DedustNativeVaultSwap<F, R> {
    pub query_id: u64,
    pub amount: BigUint,
    pub step: SwapStep,
    pub params: SwapParams<F, R>,
}

impl<F, R> CellSerialize for DedustNativeVaultSwap<F, R>
where
    F: CellSerialize,
    R: CellSerialize,
{
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder
            .pack(NATIVE_VAULT_SWAP_TAG)?
            .pack(self.query_id)?
            .pack_as::<_, &Coins>(&self.amount)?
            .store(&self.step)?
            .store_as::<_, Ref>(&self.params)?;
        Ok(())
    }
}

impl<'de, F, R> CellDeserialize<'de> for DedustNativeVaultSwap<F, R>
where
    F: CellDeserialize<'de>,
    R: CellDeserialize<'de>,
{
    fn parse(parser: &mut tlb::CellParser<'de>) -> Result<Self, CellParserError<'de>> {
        parser.unpack::<ConstU32<NATIVE_VAULT_SWAP_TAG>>()?;
        Ok(Self {
            query_id: parser.unpack()?,
            amount: parser.unpack_as::<_, Coins>()?,
            step: parser.parse()?,
            params: parser.parse_as::<_, Ref>()?,
        })
    }
}

const NATIVE_VAULT_DEPOSIT_LIQUIDITY_TAG: u32 = 0xd55e4686;

/// deposit_liquidity#d55e4686 query_id:uint64 amount:Coins pool_params:PoolParams
/// min_lp_amount:Coins
/// asset0_target_balance:Coins asset1_target_balance:Coins
/// fulfill_payload:(Maybe ^Cell)
/// reject_payload:(Maybe ^Cell) = InMsgBody;
pub struct DedustNativeVaultDepositLiquidity<F, R> {
    pub query_id: u64,
    pub amount: BigUint,
    pub pool_params: PoolParams,
    pub min_lp_amount: BigUint,
    pub target_balances: [BigUint; 2],
    pub fulfill_payload: Option<F>,
    pub reject_payload: Option<R>,
}

impl<F, R> CellSerialize for DedustNativeVaultDepositLiquidity<F, R>
where
    F: CellSerialize,
    R: CellSerialize,
{
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder
            .pack(NATIVE_VAULT_DEPOSIT_LIQUIDITY_TAG)?
            .pack(self.query_id)?
            .pack_as::<_, &Coins>(&self.amount)?
            .pack(&self.pool_params)?
            .pack_as::<_, &Coins>(&self.min_lp_amount)?
            .pack_as::<_, &[Coins; 2]>(&self.target_balances)?
            .store_as::<_, Option<Ref>>(self.fulfill_payload.as_ref())?
            .store_as::<_, Option<Ref>>(self.reject_payload.as_ref())?;
        Ok(())
    }
}

impl<'de, F, R> CellDeserialize<'de> for DedustNativeVaultDepositLiquidity<F, R>
where
    F: CellDeserialize<'de>,
    R: CellDeserialize<'de>,
{
    fn parse(parser: &mut tlb::CellParser<'de>) -> Result<Self, CellParserError<'de>> {
        parser.unpack::<ConstU32<NATIVE_VAULT_DEPOSIT_LIQUIDITY_TAG>>()?;
        Ok(Self {
            query_id: parser.unpack()?,
            amount: parser.unpack_as::<_, Coins>()?,
            pool_params: parser.unpack()?,
            min_lp_amount: parser.unpack_as::<_, Coins>()?,
            target_balances: parser.unpack_as::<_, [Coins; 2]>()?,
            fulfill_payload: parser.parse_as::<_, Option<Ref>>()?,
            reject_payload: parser.parse_as::<_, Option<Ref>>()?,
        })
    }
}

const NATIVE_VAULT_PAYOUT_TAG: u32 = 0x474f86cf;

/// payout#474f86cf query_id:uint64 payload:(Maybe ^Cell) = InMsgBody;
pub struct DedustNativeVaultPayout<P> {
    pub query_id: u64,
    pub payload: Option<P>,
}

impl<P> CellSerialize for DedustNativeVaultPayout<P>
where
    P: CellSerialize,
{
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder
            .pack(NATIVE_VAULT_PAYOUT_TAG)?
            .pack(self.query_id)?
            .store_as::<_, Option<Ref>>(self.payload.as_ref())?;
        Ok(())
    }
}

impl<'de, P> CellDeserialize<'de> for DedustNativeVaultPayout<P>
where
    P: CellDeserialize<'de>,
{
    fn parse(parser: &mut tlb::CellParser<'de>) -> Result<Self, CellParserError<'de>> {
        parser.unpack::<ConstU32<NATIVE_VAULT_PAYOUT_TAG>>()?;
        Ok(Self {
            query_id: parser.unpack()?,
            payload: parser.parse_as::<_, Option<Ref>>()?,
        })
    }
}
