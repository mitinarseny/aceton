use core::time::Duration;
use std::{collections::HashMap, sync::Arc, time::SystemTime};

use anyhow::Context;
use futures::{
    future,
    stream::{FuturesUnordered, TryStreamExt},
    try_join, TryFutureExt,
};
use num_bigint::BigUint;
use tokio::time::sleep;
use tonlib::{
    address::TonAddress,
    cell::BagOfCells,
    client::{TonClient, TonClientInterface},
    contract::{
        JettonMasterContract, TonContract, TonContractFactory, TonContractInterface,
        TonWalletContract,
    },
    wallet::TonWallet,
};
use tracing::{debug, error, info, warn};

use crate::{
    dex::{
        dedust::DEDUST_FACTORY_ADDRESS_MAINNET,
        stonfi::{PROXY_TON_ADDRESS_MAINNET, STONFI_ROUTER_ADDRESS_MAINNET},
    },
    AppConfig,
};

use super::arbitrage::TONArbitrage;

pub struct App {
    client: TonClient,
    contract_factory: TonContractFactory,

    wallet: TonWallet,
    wallet_contract: Arc<TonContract>,

    dedust_factory: Arc<TonContract>,

    stonfi_router: Arc<TonContract>,
    proxy_ton: Arc<TonContract>,
    stonfi_router_proxy_ton_wallet: Arc<TonContract>,

    ton_arbitrages: HashMap<TonAddress, TONArbitrage>,
}

impl App {
    pub async fn new(client: TonClient, cfg: AppConfig, wallet: TonWallet) -> anyhow::Result<Self> {
        let contract_factory = TonContractFactory::builder(&client)
            .build()
            .await
            .context("unable to build contract factory")?;
        let dedust_factory: Arc<TonContract> = contract_factory
            .get_contract(&DEDUST_FACTORY_ADDRESS_MAINNET)
            .into();
        let stonfi_router: Arc<TonContract> = contract_factory
            .get_contract(&STONFI_ROUTER_ADDRESS_MAINNET)
            .into();
        let proxy_ton: Arc<TonContract> = contract_factory
            .get_contract(&PROXY_TON_ADDRESS_MAINNET)
            .into();

        let stonfi_router_proxy_ton_wallet: Arc<TonContract> = contract_factory
            .get_contract(
                &proxy_ton
                    .get_wallet_address(stonfi_router.address())
                    .await?,
            )
            .into();

        Ok(Self {
            client,
            wallet_contract: contract_factory.get_contract(&wallet.address).into(),
            wallet,
            ton_arbitrages: cfg
                .jettons
                .into_iter()
                .map(|jetton_master| async {
                    let Some(arb) = TONArbitrage::new(
                        &contract_factory,
                        jetton_master.clone(),
                        dedust_factory.clone(),
                        stonfi_router.clone(),
                        stonfi_router_proxy_ton_wallet.clone(),
                    )
                    .await?
                    else {
                        warn!(%jetton_master, "not all DEXs exist");
                        return Ok(None);
                    };
                    info!(%jetton_master, "initialized");
                    anyhow::Ok(Some((jetton_master, arb)))
                })
                .collect::<FuturesUnordered<_>>()
                .try_filter_map(future::ok)
                .try_collect()
                .await?,
            contract_factory,
            dedust_factory,
            stonfi_router,
            proxy_ton,
            stonfi_router_proxy_ton_wallet,
        })
    }

    async fn round(&self) -> anyhow::Result<()> {
        const TON1: u64 = 1000000000; // 1 TON
        let (seqno, account_state) = try_join!(
            self.wallet_contract.seqno(),
            self.wallet_contract.get_account_state(),
        )?;
        let balance: u64 = account_state.balance as u64;
        info!(seqno, balance);

        if (balance as u64) < 2 * TON1 {
            error!(balance, "insufficient balance");
            return Ok(());
        }
        let ton_amount_in: BigUint = (balance - 2 * TON1).min(25 * TON1).into(); // 2 < x < 3 TON

        // 27816776064
        let opportunities = self
            .ton_arbitrages
            .values()
            .map(|arb| {
                arb.check(ton_amount_in.clone(), self.wallet.address.clone())
                    .map_err(|err| err.context(format!("check {}", arb.jetton_master.address())))
            })
            .collect::<FuturesUnordered<_>>()
            .try_filter_map(future::ok)
            .try_collect::<Vec<_>>()
            .await?;

        let Some((profit, tx)) = opportunities
            .into_iter()
            .max_by_key(|(profit, _tx)| profit.clone())
        else {
            debug!("no oppos");
            return Ok(());
        };

        info!(%profit, %ton_amount_in, "max profit");

        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let body = self
            .wallet
            .create_external_body((now + Duration::from_secs(60)).as_secs() as u32, seqno, tx)
            .context("create external message")?;
        let signed = self
            .wallet
            .sign_external_body(&body)
            .context("sign_external_body")?;
        let wrapped = self
            .wallet
            .wrap_signed_body(signed)
            .context("wrap_signed_body")?;
        let boc = BagOfCells::from_root(wrapped);
        let tx = boc.serialize(true)?;

        let tx_hash = self.client.send_raw_message_return_hash(&tx).await?;
        let tx_hash_str = hex::encode(tx_hash);
        warn!(tx.hash = tx_hash_str, "SENT TX");

        sleep(Duration::from_secs(20)).await;
        Ok(())
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        const BLOCK_INTERVAL: Duration = Duration::from_secs(3);
        loop {
            self.round().await?;

            // sleep(BLOCK_INTERVAL).await;
        }
    }
}
