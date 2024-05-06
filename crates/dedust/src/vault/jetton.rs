use num_bigint::BigUint;
use tlb::{
    BitReaderExt, BitWriterExt, CellBuilder, CellBuilderError, CellDeserialize, CellParser,
    CellParserError, CellSerialize, ConstU32, Error, Ref,
};
use tlb_ton::Coins;

use crate::{PoolParams, SwapParams, SwapStep};

const JETTON_VAULT_SWAP_TAG: u32 = 0xe3a0d482;

/// swap#e3a0d482 _:SwapStep swap_params:^SwapParams = ForwardPayload;
pub struct DedustJettonVaultSwap<F, R> {
    pub step: SwapStep,
    pub params: SwapParams<F, R>,
}

impl<F, R> CellSerialize for DedustJettonVaultSwap<F, R>
where
    F: CellSerialize,
    R: CellSerialize,
{
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder
            .pack(JETTON_VAULT_SWAP_TAG)?
            .store(&self.step)?
            .store(&self.params)?;
        Ok(())
    }
}

impl<'de, F, R> CellDeserialize<'de> for DedustJettonVaultSwap<F, R>
where
    F: CellDeserialize<'de>,
    R: CellDeserialize<'de>,
{
    fn parse(parser: &mut CellParser<'de>) -> Result<Self, CellParserError<'de>> {
        parser.unpack::<ConstU32<JETTON_VAULT_SWAP_TAG>>()?;
        Ok(Self {
            step: parser.parse()?,
            params: parser.parse()?,
        })
    }
}

const JETTON_VAULT_DEPOSIT_LIQUIDITY_TAG: u32 = 0x40e108d6;

/// deposit_liquidity#40e108d6 pool_params:PoolParams min_lp_amount:Coins
/// asset0_target_balance:Coins asset1_target_balance:Coins
/// fulfill_payload:(Maybe ^Cell)
/// reject_payload:(Maybe ^Cell) = ForwardPayload;
pub struct DedustJettonVaultDepositLiquidity<F, R> {
    pub pool_params: PoolParams,
    pub min_lp_amount: BigUint,
    pub target_balances: [BigUint; 2],
    pub fulfill_payload: Option<F>,
    pub reject_payload: Option<R>,
}

impl<F, R> CellSerialize for DedustJettonVaultDepositLiquidity<F, R>
where
    F: CellSerialize,
    R: CellSerialize,
{
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder
            .pack(JETTON_VAULT_DEPOSIT_LIQUIDITY_TAG)?
            .pack(&self.pool_params)?
            .pack_as::<_, &Coins>(&self.min_lp_amount)?
            .pack_as::<_, &[Coins; 2]>(&self.target_balances)?
            .store_as::<_, Option<Ref>>(self.fulfill_payload.as_ref())?
            .store_as::<_, Option<Ref>>(self.reject_payload.as_ref())?;
        Ok(())
    }
}

impl<'de, F, R> CellDeserialize<'de> for DedustJettonVaultDepositLiquidity<F, R>
where
    F: CellDeserialize<'de>,
    R: CellDeserialize<'de>,
{
    fn parse(parser: &mut CellParser<'de>) -> Result<Self, CellParserError<'de>> {
        parser.unpack::<ConstU32<JETTON_VAULT_SWAP_TAG>>()?;
        Ok(Self {
            pool_params: parser.unpack()?,
            min_lp_amount: parser.unpack_as::<_, Coins>()?,
            target_balances: parser.unpack_as::<_, [Coins; 2]>()?,
            fulfill_payload: parser.parse_as::<_, Option<Ref>>()?,
            reject_payload: parser.parse_as::<_, Option<Ref>>()?,
        })
    }
}
