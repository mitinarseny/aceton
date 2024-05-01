use core::{fmt::Display, str::FromStr};
use std::{error::Error as StdError, sync::Arc};

use anyhow::{anyhow, Context};
use base64::{engine::general_purpose::STANDARD, Engine};
use bitvec::view::AsBits;
use tlb::{
    BitUnpack, Cell, CellDeserializeAsOwned, CellDeserializeOwned, CellSerialize, CellSerializeAs,
    CellSerializeExt, CellSerializeWrapAsExt,
};
use tlb_ton::BoC;
use tonlibjson_client::block::{
    TvmBoxedNumber, TvmBoxedStackEntry, TvmCell, TvmNumberDecimal, TvmSlice, TvmStackEntryCell,
    TvmStackEntryNumber, TvmStackEntrySlice,
};

pub trait TvmBoxedStackEntryExt: Sized {
    fn into_boc(&self) -> anyhow::Result<BoC>;
    #[inline]
    fn into_cell(&self) -> anyhow::Result<Arc<Cell>> {
        self.into_boc()?
            .single_root()
            .context("single root")
            .cloned()
    }
    #[inline]
    fn parse_cell_fully<T>(&self) -> anyhow::Result<T>
    where
        T: CellDeserializeOwned,
    {
        self.into_cell()?.parse_fully().map_err(Into::into)
    }
    #[inline]
    fn parse_cell_fully_as<T, As>(&self) -> anyhow::Result<T>
    where
        As: CellDeserializeAsOwned<T>,
    {
        self.into_cell()?
            .parse_fully_as::<T, As>()
            .map_err(Into::into)
    }

    fn from_boc(boc: BoC) -> anyhow::Result<Self>;
    #[inline]
    fn from_cell(cell: impl Into<Arc<Cell>>) -> anyhow::Result<Self> {
        Self::from_boc(BoC::from_root(cell))
    }
    #[inline]
    fn store_cell<T>(value: T) -> anyhow::Result<Self>
    where
        T: CellSerialize,
    {
        Self::from_cell(value.to_cell()?)
    }
    fn store_cell_as<T, As>(value: T) -> anyhow::Result<Self>
    where
        As: CellSerializeAs<T>,
    {
        Self::from_cell(value.wrap_as::<As>().to_cell()?)
    }

    fn into_number<T>(&self) -> anyhow::Result<T>
    where
        T: FromStr,
        T::Err: StdError + Send + Sync + 'static;
    fn from_number<T>(number: T) -> Self
    where
        T: Display;
}

impl TvmBoxedStackEntryExt for TvmBoxedStackEntry {
    fn into_boc(&self) -> anyhow::Result<BoC> {
        let bytes = match self {
            Self::TvmStackEntrySlice(TvmStackEntrySlice {
                slice: TvmSlice { bytes },
            })
            | Self::TvmStackEntryCell(TvmStackEntryCell {
                cell: TvmCell { bytes },
            }) => bytes,
            _ => return Err(anyhow!("invalid stack")),
        };

        let bytes = STANDARD.decode(bytes).context("base64")?;

        BoC::unpack(bytes.as_bits()).map_err(Into::into)
    }

    fn from_boc(boc: BoC) -> anyhow::Result<Self> {
        Ok(Self::TvmStackEntrySlice(TvmStackEntrySlice {
            slice: TvmSlice {
                bytes: STANDARD.encode(boc.pack(true)?),
            },
        }))
    }

    fn into_number<T>(&self) -> anyhow::Result<T>
    where
        T: FromStr,
        T::Err: StdError + Send + Sync + 'static,
    {
        let Self::TvmStackEntryNumber(TvmStackEntryNumber {
            number: TvmBoxedNumber { number },
        }) = self
        else {
            return Err(anyhow!("invalid stack"));
        };

        T::from_str(number).map_err(Into::into)
    }
    fn from_number<T>(number: T) -> Self
    where
        T: Display,
    {
        Self::TvmStackEntryNumber(TvmStackEntryNumber {
            number: TvmNumberDecimal {
                number: number.to_string(),
            },
        })
    }
}
