use tonlib::cell::{CellBuilder, TonCellError};

use super::super::{SwapParams, SwapStep};

use crate::utils::tlb::{CellBuilderExt, TLBSerialize};

const JETTON_VAULT_SWAP_MESSAGE_PREFIX: u32 = 0xea06185d;
pub struct JettonVaultSwapMessage {
    pub step: SwapStep,
    pub params: SwapParams,
}

impl TLBSerialize for JettonVaultSwapMessage {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder
            .store_u32(32, JETTON_VAULT_SWAP_MESSAGE_PREFIX)?
            .store(&self.step)?
            .store_child(self.params.to_cell()?)
    }
}
