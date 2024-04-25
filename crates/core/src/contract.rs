use async_trait::async_trait;
use thiserror::Error as ThisError;
use tlb_ton::MsgAddress;
use tonlibjson_client::{
    block::{SmcRunResult, TvmBoxedStackEntry},
    ton::TonClient,
};

#[derive(Debug, ThisError)]
#[error("exit code: {0}")] // TODO
pub struct TonContractError(i32);

impl TonContractError {
    // pub const fn new(exit_code: i32) -> Self {
    //     match exit_code {
    //         0 | 1 => ,
    //         _ => Self(exit_code),
    //     }
    // }
    // fn to_str(&self) -> &'static str {
    //     match self {

    //     }
    // }
}

#[async_trait]
pub trait TonContractI {
    async fn run_get_method(
        &self,
        method: &str,
        stack: Vec<TvmBoxedStackEntry>,
    ) -> anyhow::Result<SmcRunResult>;

    async fn get(
        &self,
        method: &str,
        stack: Vec<TvmBoxedStackEntry>,
    ) -> anyhow::Result<Result<Vec<TvmBoxedStackEntry>, TonContractError>> {
        let SmcRunResult {
            stack, exit_code, ..
        } = self.run_get_method(method, stack).await?;
        Ok(match exit_code {
            0 | 1 => Ok(stack),
            _ => Err(TonContractError(exit_code)),
        })
    }
}

pub struct TonContract {
    address: MsgAddress,
    // TODO: use emulator?
    client: TonClient,
}

impl TonContract {
    pub fn address(&self) -> MsgAddress {
        self.address
    }
}

#[async_trait]
impl TonContractI for TonContract {
    async fn run_get_method(
        &self,
        method: &str,
        stack: Vec<TvmBoxedStackEntry>,
    ) -> anyhow::Result<SmcRunResult> {
        self.client
            .run_get_method(self.address.to_base64_std(), method.to_string(), stack)
            .await
    }
}
