use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr, FromInto};
use tlb::{BitPack, BitReader, BitReaderExt, BitUnpack, BitWriter, BitWriterExt, Error, NBits};
use tlb_ton::MsgAddress;
use url::Url;

#[serde_as]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum DedustAsset {
    /// native$0000 = Asset;
    Native,
    /// jetton$0001 workchain_id:int8 address:uint256 = Asset;
    Jetton(#[serde_as(as = "FromInto<AddressStruct>")] MsgAddress),
    /// extra_currency$0010 currency_id:int32 = Asset;
    #[serde(skip)]
    ExtraCurrency { currency_id: i32 },
}

impl DedustAsset {
    const NATIVE_TAG: u8 = 0b0000;
    const JETTON_TAG: u8 = 0b0001;
    const EXTRA_CURRENCY_TAG: u8 = 0b0010;
}

impl BitPack for DedustAsset {
    fn pack<W>(&self, mut writer: W) -> Result<(), W::Error>
    where
        W: BitWriter,
    {
        match self {
            DedustAsset::Native => {
                writer.pack_as::<_, NBits<4>>(Self::NATIVE_TAG)?;
            }
            DedustAsset::Jetton(addr) => {
                writer
                    .pack_as::<_, NBits<4>>(Self::JETTON_TAG)?
                    .pack_as::<_, NBits<8>>(addr.workchain_id)?
                    .pack(addr.address)?;
            }
            DedustAsset::ExtraCurrency { currency_id } => {
                writer
                    .pack_as::<_, NBits<4>>(Self::EXTRA_CURRENCY_TAG)?
                    .pack(currency_id)?;
            }
        }
        Ok(())
    }
}

impl BitUnpack for DedustAsset {
    fn unpack<R>(mut reader: R) -> Result<Self, R::Error>
    where
        R: BitReader,
    {
        Ok(match reader.unpack_as::<u8, NBits<4>>()? {
            Self::NATIVE_TAG => Self::Native,
            Self::JETTON_TAG => Self::Jetton(MsgAddress {
                workchain_id: reader.unpack::<i8>()? as i32,
                address: reader.unpack()?,
            }),
            Self::EXTRA_CURRENCY_TAG => Self::ExtraCurrency {
                currency_id: reader.unpack()?,
            },
            tag => return Err(Error::custom(format!("unknown asset tag: {tag:#06b}"))),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedustAssetWithMetadata {
    #[serde(flatten)]
    pub asset: DedustAsset,
    pub metadata: Option<DedustAssetMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedustAssetMetadata {
    pub name: String,
    pub symbol: String,
    pub image: Url,
    pub decimals: u8,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
struct AddressStruct {
    #[serde_as(as = "DisplayFromStr")]
    address: MsgAddress,
}

impl From<AddressStruct> for MsgAddress {
    fn from(AddressStruct { address }: AddressStruct) -> Self {
        address
    }
}

impl From<MsgAddress> for AddressStruct {
    fn from(address: MsgAddress) -> Self {
        Self { address }
    }
}
