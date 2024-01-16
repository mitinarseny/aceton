use async_trait::async_trait;
use tonlib::{
    address::TonAddress,
    cell::{BagOfCells, MapTonCellError},
    contract::{MapCellError, MapStackError, TonContractError, TonContractInterface},
    tl::{TvmNumber, TvmSlice, TvmStackEntry},
};

use crate::{ton_address, utils::tlb::TLBSerialize};

use super::DedustAsset;

ton_address!(pub DEDUST_FACTORY_ADDRESS_MAINNET = "EQBfBWT7X2BHg9tXAxzhz2aKiNTU1tpt5NsiK0uSDW_YAJ67");

#[async_trait]
pub trait DedustFactory: TonContractInterface {
    async fn get_vault_address(&self, asset: DedustAsset) -> Result<TonAddress, TonContractError> {
        const METHOD: &str = "get_vault_address";
        let address = self.address();
        let res = self
            .run_get_method(
                METHOD,
                &[TvmStackEntry::Slice {
                    slice: TvmSlice {
                        bytes: BagOfCells::from_root(
                            asset.to_cell().map_cell_error(METHOD, address)?,
                        )
                        .serialize(true)
                        .map_boc_serialization_error()
                        .map_cell_error(METHOD, address)?,
                    },
                }]
                .into(),
            )
            .await?;

        const STACK_SIZE: usize = 1;
        if res.stack.elements.len() != STACK_SIZE {
            return Err(TonContractError::InvalidMethodResultStackSize {
                method: METHOD.to_string(),
                address: address.clone(),
                actual: res.stack.elements.len(),
                expected: STACK_SIZE,
            });
        }

        res.stack.get_address(0).map_stack_error(METHOD, address)
    }

    async fn get_pool_address(
        &self,
        pool_type: PoolType,
        [asset0, asset1]: [DedustAsset; 2],
    ) -> Result<TonAddress, TonContractError> {
        const METHOD: &str = "get_pool_address";
        let address = self.address();
        let res = self
            .run_get_method(
                METHOD,
                &[
                    TvmStackEntry::Number {
                        number: TvmNumber {
                            number: match pool_type {
                                PoolType::Volatile => "0",
                                PoolType::Stable => "1",
                            }
                            .to_string(),
                        },
                    },
                    TvmStackEntry::Slice {
                        slice: TvmSlice {
                            bytes: BagOfCells::from_root(
                                asset0.to_cell().map_cell_error(METHOD, address)?,
                            )
                            .serialize(true)
                            .map_boc_serialization_error()
                            .map_cell_error(METHOD, address)?,
                        },
                    },
                    TvmStackEntry::Slice {
                        slice: TvmSlice {
                            bytes: BagOfCells::from_root(
                                asset1.to_cell().map_cell_error(METHOD, address)?,
                            )
                            .serialize(true)
                            .map_boc_serialization_error()
                            .map_cell_error(METHOD, address)?,
                        },
                    },
                ]
                .into(),
            )
            .await?;

        const STACK_SIZE: usize = 1;
        if res.stack.elements.len() != STACK_SIZE {
            return Err(TonContractError::InvalidMethodResultStackSize {
                method: METHOD.to_string(),
                address: address.clone(),
                actual: res.stack.elements.len(),
                expected: STACK_SIZE,
            });
        }

        res.stack.get_address(0).map_stack_error(METHOD, address)
    }
}

#[derive(Debug)]
pub enum PoolType {
    Volatile,
    Stable,
}

impl<T> DedustFactory for T where T: TonContractInterface {}
