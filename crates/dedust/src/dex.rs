use std::time::{Duration, SystemTime};

use aceton_arbitrage::{Asset, Dex, DexPool};
use aceton_core::TonContract;
use async_trait::async_trait;
use num::{traits::ConstZero, BigUint};
use tlb::CellSerializeExt;
use tlb_ton::{
    CommonMsgInfo, CurrencyCollection, ExternalInMsgInfo, ExtraCurrencyCollection, InternalMsgInfo,
    Message, MsgAddress,
};
use tonlibjson_client::ton::TonClient;
use tracing::instrument;

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
        Ok(self
            .api
            .get_available_pools()
            .await?
            .into_iter()
            // TODO
            .filter(|pool| {
                matches!(pool.r#type, DedustPoolType::Volatile)
                    && pool.reserves().iter().all(|r| **r > BigUint::from(1u64))
            })
            .collect())
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
                created_lt: 0, // TODO: ?
                created_at: 0, // TODO: ?
            }),
            init: None,
            body: DedustNativeVaultSwap {
                query_id: 0,
                amount: amount_in,
                step: steps,
                params: SwapParams {
                    deadline: (SystemTime::now() + Duration::from_secs(60))
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .expect("deadline is before UNIX_EPOCH")
                        .as_secs() as u32,
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
