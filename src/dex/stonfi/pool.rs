use async_trait::async_trait;
use num_bigint::BigUint;
use tonlib::{
    address::TonAddress,
    cell::{BagOfCells, CellBuilder, MapTonCellError},
    contract::{MapCellError, MapStackError, TonContractError, TonContractInterface},
    tl::{TvmNumber, TvmSlice, TvmStackEntry},
};

#[async_trait]
pub trait StonfiPool: TonContractInterface {
    async fn get_pool_data(&self) -> Result<PoolData, TonContractError> {
        const METHOD: &str = "get_pool_data";
        let address = self.address();
        let res = self.run_get_method(METHOD, &[].into()).await?;

        const STACK_SIZE: usize = 10;
        if res.stack.elements.len() != STACK_SIZE {
            return Err(TonContractError::InvalidMethodResultStackSize {
                method: METHOD.to_string(),
                address: address.clone(),
                actual: res.stack.elements.len(),
                expected: STACK_SIZE,
            });
        }

        Ok(PoolData {
            reserve0: res.stack.get_biguint(0).map_stack_error(METHOD, address)?,
            reserve1: res.stack.get_biguint(1).map_stack_error(METHOD, address)?,
        })
    }

    async fn get_expected_outputs(
        &self,
        jetton_wallet_in: TonAddress,
        amount_in: BigUint,
    ) -> Result<ExpectedOutputs, TonContractError> {
        const METHOD: &str = "get_expected_outputs";
        let address = self.address();
        let res = self
            .run_get_method(
                METHOD,
                &[
                    TvmStackEntry::Number {
                        number: TvmNumber {
                            number: amount_in.to_string(),
                        },
                    },
                    TvmStackEntry::Slice {
                        slice: TvmSlice {
                            bytes: BagOfCells::from_root(
                                CellBuilder::new()
                                    .store_address(&jetton_wallet_in)
                                    .map_cell_builder_error()
                                    .map_cell_error(METHOD, address)?
                                    .build()
                                    .map_cell_builder_error()
                                    .map_cell_error(METHOD, address)?,
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

        const STACK_SIZE: usize = 3;
        if res.stack.elements.len() != STACK_SIZE {
            return Err(TonContractError::InvalidMethodResultStackSize {
                method: METHOD.to_string(),
                address: address.clone(),
                actual: res.stack.elements.len(),
                expected: STACK_SIZE,
            });
        }

        Ok(ExpectedOutputs {
            jettons_to_receive: res.stack.get_biguint(0).map_stack_error(METHOD, address)?,
            protocol_fee_paid: res.stack.get_biguint(1).map_stack_error(METHOD, address)?,
            ref_fee_paid: res.stack.get_biguint(2).map_stack_error(METHOD, address)?,
        })
    }
}

impl<T> StonfiPool for T where T: TonContractInterface {}

pub struct PoolData {
    pub reserve0: BigUint,
    pub reserve1: BigUint,
}

pub struct ExpectedOutputs {
    pub jettons_to_receive: BigUint,
    pub protocol_fee_paid: BigUint,
    pub ref_fee_paid: BigUint,
}
