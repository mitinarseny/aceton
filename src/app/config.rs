use std::collections::HashSet;

use serde::Deserialize;
use tonlib::address::TonAddress;

#[derive(Deserialize)]
pub struct AppConfig {
    pub jettons: HashSet<TonAddress>,
}
