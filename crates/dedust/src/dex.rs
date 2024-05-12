use std::collections::{hash_map::Entry, HashMap};

use async_trait::async_trait;
use chrono::{Local, TimeDelta, Utc};
use futures::{
    future,
    lock::Mutex,
    stream::{self, FuturesUnordered},
    StreamExt, TryFutureExt, TryStreamExt,
};
use num::{BigUint, One};
use tlb::CellSerializeExt;
use tlb_ton::{
    CommonMsgInfo, CurrencyCollection, ExtraCurrencyCollection, InternalMsgInfo, Message,
    MsgAddress,
};
use tonlibjson_client::ton::TonClient;
use tracing::{debug, info, instrument};

use aceton_arbitrage::{Asset, Dex, DexPool};
use aceton_core::TonContract;

use crate::{
    api::DedustHTTPClient, factory, DedustFactoryI, DedustNativeVaultSwap, DedustPool, DedustPoolI,
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

#[async_trait]
impl Dex for DeDust {
    type Pool = DedustPool;

    #[instrument(skip(self))]
    async fn get_pools(&self) -> anyhow::Result<Vec<Self::Pool>> {
        const MAX_TRADE_AGE: TimeDelta = TimeDelta::days(10);

        stream::iter(
            self.api
                .get_available_pools()
                .await?
                .into_iter()
                .filter(|pool| {
                    // TODO
                    matches!(pool.r#type, DedustPoolType::Volatile)
                        && pool.reserves().into_iter().all(|r| r > &BigUint::one())
                })
                .map({
                    let now = Local::now();
                    move |pool| async move {
                        let latest_trades = self.api.get_latest_trades(pool.address, 1).await?;
                        let Some(last_trade) = latest_trades.last() else {
                            return Ok(None);
                        };

                        if now.signed_duration_since(last_trade.created_at) > MAX_TRADE_AGE {
                            return Ok(None);
                        }
                        Ok(Some(pool))
                    }
                }),
        )
        .buffer_unordered(100)
        .try_filter_map(future::ok)
        .try_collect()
        .await
    }

    #[instrument(skip(self), fields(%pool.address))]
    async fn update_pool(&self, pool: &mut Self::Pool) -> anyhow::Result<()> {
        let pool_contract = TonContract::new(self.ton_client.clone(), pool.address);
        let new_reserves = pool_contract.get_reserves().await?;
        if new_reserves != pool.reserves {
            debug!(
                old_reserves = ?pool.reserves,
                ?new_reserves,
                "pool updated",
            );
            pool.reserves = new_reserves
        }
        Ok(())
    }

    async fn make_message(
        &self,
        query_id: u64,
        asset_in: Asset,
        amount_in: BigUint,
        steps: <Self::Pool as DexPool>::Step,
    ) -> anyhow::Result<Message> {
        let now = Local::now().with_timezone(&Utc);
        // TODO: get vault address from factory
        Ok(Message {
            info: CommonMsgInfo::Internal(InternalMsgInfo {
                ihr_disabled: true,
                bounce: true,
                bounced: false,
                src: MsgAddress::NULL,
                dst: self.vault_address(asset_in).await?,
                value: CurrencyCollection {
                    grams: amount_in.clone(), // TODO: + on network fees
                    other: ExtraCurrencyCollection,
                },
                ihr_fee: BigUint::ZERO,
                fwd_fee: BigUint::ZERO,
                created_lt: 0, // TODO: ?
                created_at: now,
            }),
            init: None,
            body: DedustNativeVaultSwap {
                query_id,
                amount: amount_in,
                step: steps,
                params: SwapParams {
                    deadline: now + TimeDelta::seconds(30),
                    recepient: MsgAddress::NULL, // TODO: self addr or STONFI?
                    referral: MsgAddress::NULL,
                    fulfill_payload: Option::<()>::None, // TODO: STONFI swap msg?
                    reject_payload: Option::<()>::None,
                },
            }
            .to_cell()
            .unwrap(), // TODO: err
        })
    }
}
