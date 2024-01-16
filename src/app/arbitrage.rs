use core::time::Duration;
use std::{sync::Arc, time::SystemTime};

use anyhow::{anyhow, Context};
use futures::try_join;
use num_bigint::{BigInt, BigUint};
use tonlib::{
    address::TonAddress,
    cell::Cell,
    contract::{JettonMasterContract, TonContract, TonContractFactory, TonContractInterface},
    message::TransferMessage,
};
use tracing::{debug, instrument, warn};

use crate::{
    dex::{
        dedust::{
            DedustAsset, DedustFactory, DedustPool, NativeVaultSwapMessage, PoolType, SwapKind,
            SwapParams, SwapStep, SwapStepParams,
        },
        stonfi::{StonfiPool, StonfiRouter, StonfiRouterSwapMessage},
    },
    utils::tlb::TLBSerialize,
};

pub struct TONArbitrage {
    pub jetton_master: Arc<TonContract>,

    pub dedust_pool: Arc<TonContract>,
    pub dedust_native_vault: Arc<TonContract>,
    pub dedust_jetton_vault: Arc<TonContract>,

    pub stonfi_router: Arc<TonContract>,
    pub stonfi_pool: Arc<TonContract>,
    pub stonfi_router_proxy_ton_wallet: Arc<TonContract>,
    pub stonfi_router_jetton_wallet: Arc<TonContract>,
}

impl TONArbitrage {
    pub async fn new(
        contract_factory: &TonContractFactory,
        jetton_master: TonAddress,
        dedust_factory: impl Into<Arc<TonContract>>,
        stonfi_router: impl Into<Arc<TonContract>>,
        stonfi_router_proxy_ton_wallet: impl Into<Arc<TonContract>>,
    ) -> anyhow::Result<Option<Self>> {
        let dedust_factory: Arc<TonContract> = dedust_factory.into();
        let stonfi_router: Arc<TonContract> = stonfi_router.into();
        let stonfi_router_proxy_ton_wallet: Arc<TonContract> =
            stonfi_router_proxy_ton_wallet.into();
        let jetton_master = contract_factory.get_contract(&jetton_master);
        let Ok((dedust_pool, stonfi_router_jetton_wallet)) = try_join!(
            DedustFactory::get_pool_address(
                &*dedust_factory,
                PoolType::Volatile,
                [
                    DedustAsset::Native,
                    DedustAsset::Jetton(jetton_master.address().clone())
                ]
            ),
            jetton_master.get_wallet_address(stonfi_router.address()),
        ) else {
            return Ok(None);
        };
        let dedust_pool = contract_factory.get_contract(&dedust_pool);
        let Ok((dedust_native_vault, dedust_jetton_vault)) = try_join!(
            dedust_factory.get_vault_address(DedustAsset::Native),
            dedust_factory.get_vault_address(DedustAsset::Jetton(jetton_master.address().clone())),
        ) else {
            return Ok(None);
        };

        let Ok(stonfi_pool) = StonfiRouter::get_pool_address(
            &*stonfi_router,
            [
                stonfi_router_proxy_ton_wallet.address().clone(),
                stonfi_router_jetton_wallet.clone(),
            ],
        )
        .await
        else {
            return Ok(None);
        };
        let stonfi_pool = contract_factory.get_contract(&stonfi_pool);

        const TON1: u64 = 1000000000u64;
        if let Err(_) = try_join!(
            DedustPool::estimate_swap_out(&dedust_pool, DedustAsset::Native, TON1.into()),
            StonfiPool::get_expected_outputs(
                &stonfi_pool,
                stonfi_router_proxy_ton_wallet.address().clone(),
                TON1.into()
            ),
        ) {
            return Ok(None);
        }

        Ok(Some(Self {
            jetton_master: jetton_master.into(),
            dedust_pool: dedust_pool.into(),
            dedust_native_vault: contract_factory.get_contract(&dedust_native_vault).into(),
            dedust_jetton_vault: contract_factory.get_contract(&dedust_jetton_vault).into(),
            stonfi_router,
            stonfi_router_proxy_ton_wallet,
            stonfi_router_jetton_wallet: contract_factory
                .get_contract(&stonfi_router_jetton_wallet)
                .into(),
            stonfi_pool: stonfi_pool.into(),
        }))
    }

    pub async fn dedust_swap_ton(&self, ton_amount_in: BigUint) -> anyhow::Result<BigUint> {
        let result = self
            .dedust_pool
            .estimate_swap_out(DedustAsset::Native, ton_amount_in.clone())
            .await
            .context("estimate_swap_out")?;

        if result.asset_out != DedustAsset::Jetton(self.jetton_master.address().clone()) {
            return Err(anyhow!("output differs from expected jetton"));
        }

        Ok(result.amount_out)
    }

    pub async fn dedust_swap_jetton(&self, jetton_amount_in: BigUint) -> anyhow::Result<BigUint> {
        let result = self
            .dedust_pool
            .estimate_swap_out(
                DedustAsset::Jetton(self.jetton_master.address().clone()),
                jetton_amount_in.clone(),
            )
            .await
            .context("estimate_swap_out")?;

        if result.asset_out != DedustAsset::Native {
            return Err(anyhow!("output is not Native"));
        }

        Ok(result.amount_out)
    }

    pub async fn stonfi_swap_ton(&self, ton_amount_in: BigUint) -> anyhow::Result<BigUint> {
        let res = self
            .stonfi_pool
            .get_expected_outputs(
                self.stonfi_router_proxy_ton_wallet.address().clone(),
                ton_amount_in,
            )
            .await
            .context("get_expected_outputs")?;

        Ok(res.jettons_to_receive)
    }

    pub async fn stonfi_swap_jetton(&self, jetton_amount_in: BigUint) -> anyhow::Result<BigUint> {
        let res = self
            .stonfi_pool
            .get_expected_outputs(
                self.stonfi_router_jetton_wallet.address().clone(),
                jetton_amount_in,
            )
            .await
            .context("get_expected_outputs")?;

        Ok(res.jettons_to_receive)
    }

    #[instrument(skip_all, fields(
        jetton_master = %self.jetton_master.address(),
    ))]
    pub async fn check(
        &self,
        ton_amount_in: BigUint,
        self_address: TonAddress,
    ) -> anyhow::Result<Option<(Profit, Cell)>> {
        const SLIPPAGE_PERCENT: u8 = 5; // 5%
        const GAS: u64 = 200000000u64; // 0.17 TON
        let dedust_amount_out = self.dedust_swap_ton(ton_amount_in.clone()).await?;
        // TODO: add all amount that we have on other jetton
        let ton_amount_out = self.stonfi_swap_jetton(dedust_amount_out.clone()).await?;
        let profit =
            BigInt::from(ton_amount_out.clone()) - BigInt::from(ton_amount_in.clone()) - GAS;

        if profit <= 0.into() {
            debug!(%ton_amount_out, %ton_amount_in, %profit, "non-profitable");
            return Ok(None);
        }

        warn!(%ton_amount_out, %ton_amount_in, %profit, "PROFITABLE");

        Ok(Some((
            profit,
            TransferMessage::new(
                self.dedust_native_vault.address(),
                &(ton_amount_in.clone() + GAS),
            )
            .with_data(
                NativeVaultSwapMessage {
                    query_id: 0,
                    amount: ton_amount_in,
                    step: SwapStep {
                        pool: self.dedust_pool.address().clone(),
                        params: SwapStepParams {
                            kind: SwapKind::GivenIn,
                            // TODO: gas
                            limit: dedust_amount_out.clone()
                                - ((dedust_amount_out.clone() * 100u32)
                                    / (SLIPPAGE_PERCENT as u32 * 100u32)), // TODO
                            next: None,
                        },
                    },
                    params: SwapParams {
                        // TODO: deadline
                        deadline: SystemTime::now() + Duration::from_secs(60),
                        recepient: self.stonfi_router.address().clone(),
                        referral: TonAddress::NULL,
                        fulfill_payload: Some(
                            StonfiRouterSwapMessage {
                                token_wallet1: self
                                    .stonfi_router_proxy_ton_wallet
                                    .address()
                                    .clone(),
                                // TODO: maybe 0?
                                min_out: 0u64.into(),
                                to_address: self_address,
                                referral: None,
                            }
                            .to_cell()?,
                        ),
                        reject_payload: None,
                    },
                }
                .to_cell()?,
            )
            .build()?,
        )))
    }
}

pub type Profit = BigInt;
