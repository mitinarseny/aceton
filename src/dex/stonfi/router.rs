use async_trait::async_trait;
use futures::try_join;
use num_bigint::BigUint;
use tonlib::{
    address::TonAddress,
    cell::{BagOfCells, CellBuilder, MapTonCellError, TonCellError},
    contract::{
        JettonMasterContract, MapCellError, MapStackError, TonContractError, TonContractInterface,
    },
    tl::{TvmSlice, TvmStackEntry},
};

use crate::{
    ton_address,
    utils::tlb::{CellBuilderExt, TLBSerialize},
};

ton_address!(pub STONFI_ROUTER_ADDRESS_MAINNET = "EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt");

#[async_trait]
pub trait StonfiRouter: TonContractInterface {
    async fn get_pool_address(
        &self,
        [jetton0_wallet, jetton1_wallet]: [TonAddress; 2],
    ) -> Result<TonAddress, TonContractError> {
        const METHOD: &str = "get_pool_address";
        let address = self.address();
        let res = self
            .run_get_method(
                METHOD,
                &[
                    TvmStackEntry::Slice {
                        slice: TvmSlice {
                            bytes: BagOfCells::from_root(
                                CellBuilder::new()
                                    .store_address(&jetton0_wallet)
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
                    TvmStackEntry::Slice {
                        slice: TvmSlice {
                            bytes: BagOfCells::from_root(
                                CellBuilder::new()
                                    .store_address(&jetton1_wallet)
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

    async fn get_pool_address_by_jettons<J>(
        &self,
        [jetton0_master, jetton1_master]: [J; 2],
    ) -> Result<TonAddress, TonContractError>
    where
        J: JettonMasterContract + Send + Sync,
    {
        let address = self.address();
        let (jetton0_wallet, jetton1_wallet) = try_join!(
            jetton0_master.get_wallet_address(address),
            jetton1_master.get_wallet_address(address),
        )?;
        self.get_pool_address([jetton0_wallet, jetton1_wallet])
            .await
    }
}

impl<T> StonfiRouter for T where T: TonContractInterface {}

const SWAP_OP: u32 = 0x25938561;
pub struct StonfiRouterSwapMessage {
    pub token_wallet1: TonAddress,
    pub min_out: BigUint,
    pub to_address: TonAddress,
    pub referral: Option<TonAddress>,
}

impl TLBSerialize for StonfiRouterSwapMessage {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder
            .store_u32(32, SWAP_OP)?
            .store_address(&self.token_wallet1)?
            .store_coins(&self.min_out)?
            .store_address(&self.to_address)?
            .store(&self.referral)
    }
}
