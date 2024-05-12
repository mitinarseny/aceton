use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use chrono::{Local, TimeDelta, Utc};
use futures::{future, stream, StreamExt, TryStreamExt};
use num::{BigUint, One};
use tlb::CellSerializeExt;
use tlb_ton::{
    CommonMsgInfo, CurrencyCollection, ExtraCurrencyCollection, InternalMsgInfo, Message,
    MsgAddress,
};
use tonlibjson_client::ton::TonClient;
use tracing::instrument;

use aceton_arbitrage::{Asset, Dex, DexPool};
use aceton_core::TonContract;

use crate::{
    api::DedustHTTPClient, DedustNativeVaultSwap, DedustPool, DedustPoolI, DedustPoolType,
    SwapParams,
};

pub struct DeDust {
    ton_client: TonClient,
    api: DedustHTTPClient,
}

impl DeDust {
    pub fn new(ton_client: TonClient, http_client: reqwest::Client) -> Self {
        Self {
            ton_client,
            api: DedustHTTPClient::new(http_client),
        }
    }
}

#[async_trait]
impl Dex for DeDust {
    type Pool = DedustPool;

    #[instrument(skip(self))]
    async fn get_pools(&self) -> anyhow::Result<Vec<Self::Pool>> {
        const MAX_TRADE_AGE: TimeDelta = TimeDelta::days(10);

        let now = Local::now();
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
                .map(|pool| async {
                    let latest_trades = self.api.get_latest_trades(pool.address, 1).await?;
                    let Some(last_trade) = latest_trades.last() else {
                        return Ok(None);
                    };

                    if now.signed_duration_since(last_trade.created_at) > MAX_TRADE_AGE {
                        return Ok(None);
                    }
                    Ok(Some(pool))
                }),
        )
        .buffer_unordered(100)
        .try_filter_map(future::ok)
        .try_collect()
        .await
    }

    async fn update_pool(&self, pool: &mut Self::Pool) -> anyhow::Result<()> {
        let pool_contract = TonContract::new(self.ton_client.clone(), pool.address);
        pool.reserves = pool_contract.get_reserves().await?;
        Ok(())
    }

    fn make_message(
        &self,
        asset_in: Asset,
        amount_in: BigUint,
        steps: <Self::Pool as DexPool>::Step,
    ) -> Message {
        // TODO: get vault address from factory
        Message {
            info: CommonMsgInfo::Internal(InternalMsgInfo {
                ihr_disabled: true,
                bounce: true,
                bounced: false,
                src: MsgAddress::NULL,
                dst: MsgAddress::NULL, // TODO: vault address
                value: CurrencyCollection {
                    grams: amount_in.clone(), // TODO: + on network fees
                    other: ExtraCurrencyCollection,
                },
                ihr_fee: BigUint::ZERO,
                fwd_fee: BigUint::ZERO,
                created_lt: 0,       // TODO: ?
                created_at: todo!(), // TODO: ?
            }),
            init: None,
            body: DedustNativeVaultSwap {
                query_id: 0,
                amount: amount_in,
                step: steps,
                params: SwapParams {
                    deadline: Local::now().with_timezone(&Utc),
                    recepient: MsgAddress::NULL, // TODO: self addr or STONFI?
                    referral: MsgAddress::NULL,
                    fulfill_payload: Option::<()>::None, // TODO: STONFI swap msg?
                    reject_payload: Option::<()>::None,
                },
            }
            .to_cell()
            .unwrap(), // TODO: err
        }
    }
}
