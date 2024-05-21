use aceton_core::Asset;
use tlb::{BitPackAs, BitReader, BitReaderExt, BitUnpackAs, BitWriter, BitWriterExt, Error, NBits};
use tlb_ton::MsgAddress;

pub struct DedustAsset;

impl DedustAsset {
    const NATIVE_TAG: u8 = 0b0000;
    const JETTON_TAG: u8 = 0b0001;
    const EXTRA_CURRENCY_TAG: u8 = 0b0010;
}

impl BitPackAs<Asset> for DedustAsset {
    fn pack_as<W>(source: &Asset, mut writer: W) -> Result<(), W::Error>
    where
        W: BitWriter,
    {
        match source {
            Asset::Native => {
                writer.pack_as::<_, NBits<4>>(Self::NATIVE_TAG)?;
            }
            Asset::Jetton(addr) => {
                writer
                    .pack_as::<_, NBits<4>>(Self::JETTON_TAG)?
                    .pack_as::<_, NBits<8>>(addr.workchain_id)?
                    .pack(addr.address)?;
            }
            Asset::ExtraCurrency { currency_id } => {
                writer
                    .pack_as::<_, NBits<4>>(Self::EXTRA_CURRENCY_TAG)?
                    .pack(currency_id)?;
            }
        }
        Ok(())
    }
}

impl BitUnpackAs<Asset> for DedustAsset {
    fn unpack_as<R>(mut reader: R) -> Result<Asset, R::Error>
    where
        R: BitReader,
    {
        Ok(match reader.unpack_as::<u8, NBits<4>>()? {
            Self::NATIVE_TAG => Asset::Native,
            Self::JETTON_TAG => Asset::Jetton(MsgAddress {
                workchain_id: reader.unpack::<i8>()? as i32,
                address: reader.unpack()?,
            }),
            Self::EXTRA_CURRENCY_TAG => Asset::ExtraCurrency {
                currency_id: reader.unpack()?,
            },
            tag => return Err(Error::custom(format!("unknown asset tag: {tag:#06b}"))),
        })
    }
}
