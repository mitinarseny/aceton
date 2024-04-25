use tonlibjson_client::ton::TonClient;

pub struct Arbitrager {
    ton: TonClient,
}

impl Arbitrager {
    pub fn run(&self) -> anyhow::Result<()> {
        // self.ton.get_account_tx_stream("")
        Ok(())
    }
}
