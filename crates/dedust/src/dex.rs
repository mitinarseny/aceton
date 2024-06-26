use std::collections::{hash_map::Entry, HashMap};

use aceton_core::{ton_utils::contract::TonContract, Asset, Dex, DexBody, DexPool};
use async_trait::async_trait;
use chrono::{Local, TimeDelta};
use futures::{
    future,
    lock::Mutex,
    stream::{self},
    StreamExt, TryStreamExt,
};
use lazy_static::lazy_static;
use num::{BigUint, One};
use tlb_ton::MsgAddress;
use tonlibjson_client::ton::TonClient;
use tracing::{debug, instrument};

use crate::{
    api::DedustHTTPClient, DedustFactoryI, DedustNativeVaultSwap, DedustPool, DedustPoolI,
    DedustPoolType, SwapParams,
};

pub struct DeDust {
    ton_client: TonClient,
    api: DedustHTTPClient,
    factory: MsgAddress,
    vaults: Mutex<HashMap<Asset, MsgAddress>>,
}

impl DeDust {
    pub fn new(ton_client: TonClient, factory: MsgAddress, http_client: reqwest::Client) -> Self {
        Self {
            ton_client,
            factory,
            api: DedustHTTPClient::new(http_client),
            vaults: Default::default(),
        }
    }

    #[instrument(skip(self))]
    async fn vault_address(&self, asset: Asset) -> anyhow::Result<MsgAddress> {
        let mut vaults = self.vaults.lock().await;
        Ok(match vaults.entry(asset) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let factory = TonContract::new(self.ton_client.clone(), self.factory);
                let address = factory.get_vault_address(asset).await?;
                debug!(%asset, %address, "resolved vault address");
                *entry.insert(address)
            }
        })
    }
}

lazy_static! {
    // 0.0225 TON ~= 0.025 TON
    static ref SWAP_STEP_GAS: BigUint = 22_500_000u32.into();
    static ref SWAP_EXTERNAL_PAYOUT: BigUint = (
        100_000_000u32 + // swap_external: 0.08 TON ~= 0.1 TON
        100_000_000u32 // payout: 0.09 TON ~= 0.1 TON
    ).into();
}

#[async_trait]
impl Dex for DeDust {
    type Pool = DedustPool;
    type Body = DedustNativeVaultSwap<(), ()>;

    #[instrument(skip(self))]
    async fn get_pools(&self) -> anyhow::Result<Vec<Self::Pool>> {
        // const MAX_TRADE_AGE: TimeDelta = TimeDelta::days(300);

        stream::iter(
            self.api
                .get_available_pools()
                .await?
                .into_iter()
                .filter(|pool| {
                    // TODO
                    // matches!(pool.r#type, DedustPoolType::Volatile)
                    pool.reserves().into_iter().all(|r| r > &BigUint::one())
                })
                .map({
                    let now = Local::now();
                    move |pool| async move {
                        let latest_trades = self.api.get_latest_trades(pool.address, 1).await?;
                        let Some(last_trade) = latest_trades.last() else {
                            return Ok(None);
                        };

                        // if now.signed_duration_since(last_trade.created_at) > MAX_TRADE_AGE {
                        //     return Ok(None);
                        // }
                        Ok(Some(pool))
                    }
                }),
        )
        .buffer_unordered(100)
        .try_filter_map(future::ok)
        .try_collect()
        .await
    }

    #[instrument(skip_all, fields(%pool.address))]
    async fn update_pool(&self, pool: &mut Self::Pool) -> anyhow::Result<bool> {
        let pool_contract = TonContract::new(self.ton_client.clone(), pool.address);
        let new_reserves = pool_contract.get_reserves().await?;
        let is_updated = pool.reserves != new_reserves;
        pool.reserves = new_reserves;
        if is_updated {
            debug!("pool updated");
        }
        Ok(is_updated)
    }

    async fn make_body(
        &self,
        query_id: u64,
        asset_in: Asset,
        amount_in: BigUint,
        steps: <Self::Pool as DexPool>::Step,
    ) -> anyhow::Result<DexBody<Self::Body>> {
        Ok(DexBody {
            dst: self.vault_address(asset_in).await?,
            gas: &*SWAP_EXTERNAL_PAYOUT + &*SWAP_STEP_GAS * steps.len(),
            body: DedustNativeVaultSwap {
                query_id,
                amount: amount_in,
                step: steps,
                params: SwapParams {
                    deadline: None,
                    recepient: MsgAddress::NULL,
                    referral: MsgAddress::NULL,
                    fulfill_payload: Option::<()>::None,
                    reject_payload: Option::<()>::None,
                },
            },
        })
    }
}
