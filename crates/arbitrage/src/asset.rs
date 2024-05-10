use core::fmt::{self, Debug, Display};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr, FromInto};
use tlb_ton::MsgAddress;
use url::Url;

#[serde_as]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Asset {
    Native,
    Jetton(
        /// Address of jetton_master
        #[serde_as(as = "FromInto<AddressField>")]
        MsgAddress,
    ),
    ExtraCurrency {
        currency_id: i32,
    },
}

impl Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self, f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub name: String,
    pub symbol: String,
    pub image: Option<Url>,
    pub decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetWithMetadata {
    #[serde(flatten)]
    pub asset: Asset,
    pub metadata: Option<AssetMetadata>,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
struct AddressField {
    #[serde_as(as = "DisplayFromStr")]
    address: MsgAddress,
}

impl From<AddressField> for MsgAddress {
    fn from(AddressField { address }: AddressField) -> Self {
        address
    }
}

impl From<MsgAddress> for AddressField {
    fn from(address: MsgAddress) -> Self {
        Self { address }
    }
}
