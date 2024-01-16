use async_trait::async_trait;
use num_bigint::BigUint;
use tonlib::{
    cell::{BagOfCells, MapTonCellError},
    contract::{MapCellError, MapStackError, TonContractError, TonContractInterface},
    tl::{TvmNumber, TvmSlice, TvmStackEntry},
};

use crate::utils::tlb::{CellExt, TLBSerialize};

use super::DedustAsset;

#[async_trait]
pub trait DedustPool: TonContractInterface {
    async fn get_assets(&self) -> Result<[DedustAsset; 2], TonContractError> {
        const METHOD: &str = "get_assets";
        let address = self.address();
        let res = self.run_get_method(METHOD, &[].into()).await?;

        const STACK_SIZE: usize = 2;
        if res.stack.elements.len() != STACK_SIZE {
            return Err(TonContractError::InvalidMethodResultStackSize {
                method: METHOD.to_string(),
                address: address.clone(),
                actual: res.stack.elements.len(),
                expected: STACK_SIZE,
            });
        }

        let asset0 = res
            .stack
            .get_boc(0)
            .map_stack_error(METHOD, address)?
            .single_root()
            .map_cell_error(METHOD, address)?
            .parse_to_fully()
            .map_cell_error(METHOD, address)?;
        let asset1 = res
            .stack
            .get_boc(1)
            .map_stack_error(METHOD, address)?
            .single_root()
            .map_cell_error(METHOD, address)?
            .parse_to_fully()
            .map_cell_error(METHOD, address)?;

        Ok([asset0, asset1])
    }

    async fn get_reserves(&self) -> Result<[BigUint; 2], TonContractError> {
        const METHOD: &str = "get_reserves";
        let address = self.address();
        let res = self.run_get_method(METHOD, &[].into()).await?;

        const STACK_SIZE: usize = 2;
        if res.stack.elements.len() != STACK_SIZE {
            return Err(TonContractError::InvalidMethodResultStackSize {
                method: METHOD.to_string(),
                address: address.clone(),
                actual: res.stack.elements.len(),
                expected: STACK_SIZE,
            });
        }

        let reserve0 = res.stack.get_biguint(0).map_stack_error(METHOD, address)?;
        let reserve1 = res.stack.get_biguint(1).map_stack_error(METHOD, address)?;
        Ok([reserve0, reserve1])
    }

    async fn estimate_swap_out(
        &self,
        asset_in: DedustAsset,
        amount_in: BigUint,
    ) -> Result<EstimateSwapOutOutput, TonContractError> {
        const METHOD: &str = "estimate_swap_out";
        let address = self.address();
        let res = self
            .run_get_method(
                METHOD,
                &[
                    TvmStackEntry::Slice {
                        slice: TvmSlice {
                            bytes: BagOfCells::from_root(
                                asset_in.to_cell().map_cell_error(METHOD, address)?,
                            )
                            .serialize(true)
                            .map_boc_serialization_error()
                            .map_cell_error(METHOD, address)?,
                        },
                    },
                    TvmStackEntry::Number {
                        number: TvmNumber {
                            number: amount_in.to_string(),
                        },
                    },
                ]
                .into(),
            )
            .await?;

        const STACK_SIZE: usize = 3;
        if res.stack.elements.len() != STACK_SIZE {
            return Err(TonContractError::InvalidMethodResultStackSize {
                method: METHOD.to_string(),
                address: address.clone(),
                actual: res.stack.elements.len(),
                expected: STACK_SIZE,
            });
        }

        Ok(EstimateSwapOutOutput {
            asset_out: res
                .stack
                .get_boc(0)
                .map_stack_error(METHOD, address)?
                .single_root()
                .map_cell_error(METHOD, address)?
                .parse_to_fully()
                .map_cell_error(METHOD, address)?,
            amount_out: res.stack.get_biguint(1).map_stack_error(METHOD, address)?,
            trade_fee: res.stack.get_biguint(2).map_stack_error(METHOD, address)?,
        })
    }
}

impl<T> DedustPool for T where T: TonContractInterface {}

#[derive(Debug, Clone)]
pub struct EstimateSwapOutOutput {
    pub asset_out: DedustAsset,
    /// amount of `asset_out``
    pub amount_out: BigUint,
    /// amount of `asset_in`` asset given as a fee
    pub trade_fee: BigUint,
}
