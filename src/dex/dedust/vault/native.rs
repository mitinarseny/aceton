use num_bigint::BigUint;
use tonlib::cell::{CellBuilder, TonCellError};

use super::super::{SwapParams, SwapStep};

use crate::utils::tlb::{CellBuilderExt, TLBSerialize};

const NATIVE_VAULT_SWAP_MESSAGE_PREFIX: u32 = 0xea06185d;
pub struct NativeVaultSwapMessage {
    pub query_id: u64,
    pub amount: BigUint,
    pub step: SwapStep,
    pub params: SwapParams,
}

impl TLBSerialize for NativeVaultSwapMessage {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder
            .store_u32(32, NATIVE_VAULT_SWAP_MESSAGE_PREFIX)?
            .store_u64(64, self.query_id)?
            .store_coins(&self.amount)?
            .store(&self.step)?
            .store(self.params.to_ref_cell()?)
    }
}
