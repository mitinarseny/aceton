use tonlib::{
    address::TonAddress,
    cell::{CellBuilder, CellParser, TonCellError},
};

use crate::utils::tlb::{TLBDeserialize, TLBSerialize};

/// https://docs.dedust.io/reference/tlb-schemes#asset
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedustAsset {
    Native,
    Jetton(TonAddress),
    ExtraCurrency { currency_id: i32 },
}

impl TLBSerialize for DedustAsset {
    fn store<'a>(&'a self, builder: &'a mut CellBuilder) -> Result<&'a mut CellBuilder, TonCellError> {
        match self {
            DedustAsset::Native => builder.store_u8(4, 0b0000),
            DedustAsset::Jetton(address) => builder
                .store_u8(4, 0b0001)?
                .store_i8(8, address.workchain as i8)?
                .store_slice(&address.hash_part),
            DedustAsset::ExtraCurrency { currency_id } => builder
                .store_u32(4, 0b0010)?
                .store_u32(32, *currency_id as u32),
        }
    }
}

impl TLBDeserialize for DedustAsset {
    fn load(parser: &mut CellParser) -> Result<Self, TonCellError> {
        let prefix = parser.load_u8(4)?;
        match prefix {
            0b0000 => Ok(Self::Native),
            0b0001 => Ok(Self::Jetton(TonAddress {
                workchain: (parser.load_u8(8)? as i8) as i32,
                hash_part: {
                    let mut address = [0; 32];
                    parser.load_slice(&mut address)?;
                    address
                },
            })),
            0b0010 => Ok(Self::ExtraCurrency {
                currency_id: parser.load_u32(32)? as i32,
            }),
            _ => Err(TonCellError::cell_parser_error(format!(
                "unknown prefix: {prefix:#b}"
            ))),
        }
    }
}
