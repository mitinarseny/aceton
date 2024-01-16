use std::time::SystemTime;

use num_bigint::BigUint;
use tonlib::{
    address::TonAddress,
    cell::{Cell, CellBuilder, TonCellError},
};

use crate::utils::tlb::{CellBuilderExt, RefCell, TLBSerialize};

pub struct SwapParams {
    pub deadline: SystemTime,
    pub recepient: TonAddress,
    pub referral: TonAddress,
    pub fulfill_payload: Option<Cell>,
    pub reject_payload: Option<Cell>,
}

impl TLBSerialize for SwapParams {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder
            .store_u32(
                32,
                self.deadline
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("deadline is before UNIX_EPOCH")
                    .as_secs() as u32,
            )?
            .store_address(&self.recepient)?
            .store_address(&self.referral)?
            .store(self.fulfill_payload.clone().map(RefCell::from))?
            .store(self.reject_payload.clone().map(RefCell::from))
    }
}

pub struct SwapStep {
    pub pool: TonAddress,
    pub params: SwapStepParams,
}

impl TLBSerialize for SwapStep {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder.store_address(&self.pool)?.store(&self.params)
    }
}

pub struct SwapStepParams {
    pub kind: SwapKind,
    pub limit: BigUint,
    pub next: Option<Box<SwapStep>>,
}

impl TLBSerialize for SwapStepParams {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder.store(self.kind)?.store_coins(&self.limit)?.store(
            self.next
                .as_ref()
                .map(|next| next.to_ref_cell())
                .transpose()?,
        )
    }
}

#[derive(Clone, Copy)]
pub enum SwapKind {
    GivenIn,
    // TODO: Not Implemented
    // GivenOut,
}

impl TLBSerialize for SwapKind {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder.store_bit(match self {
            SwapKind::GivenIn => false,
        })
    }
}
