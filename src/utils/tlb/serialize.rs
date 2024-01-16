use std::sync::Arc;

use impl_tools::autoimpl;
use num_bigint::BigUint;
use tonlib::{
    address::TonAddress,
    cell::{Cell, CellBuilder, TonCellError},
};

use super::{NumRepr, RefCell};

#[autoimpl(for<T: trait> &T, &mut T, Box<T>, Arc<T>)]
pub trait TLBSerialize: Sized {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError>;

    fn to_cell(&self) -> Result<Cell, TonCellError> {
        let mut builder = CellBuilder::new();
        self.store(&mut builder)?;
        builder.build()
    }

    fn to_ref_cell(&self) -> Result<RefCell, TonCellError> {
        self.to_cell().map(Into::into)
    }
}

pub trait CellBuilderExt {
    fn store<T>(&mut self, value: T) -> Result<&mut Self, TonCellError>
    where
        T: TLBSerialize;
}

impl CellBuilderExt for CellBuilder {
    fn store<T>(&mut self, value: T) -> Result<&mut Self, TonCellError>
    where
        T: TLBSerialize,
    {
        value.store(self)?;
        Ok(self)
    }
}

/// [Maybe](https://docs.ton.org/develop/data-formats/tl-b-types#maybe)
impl<T> TLBSerialize for Option<T>
where
    T: TLBSerialize,
{
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        match self {
            None => builder.store_bit(false),
            Some(v) => builder.store_bit(true)?.store(v),
        }
    }
}

impl TLBSerialize for Cell {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder.store_cell(self)
    }
}

impl TLBSerialize for RefCell {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder.store_reference(&self.0)
    }
}

impl<const N: usize> TLBSerialize for NumRepr<u8, N> {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder.store_u8(N, self.0)
    }
}

impl<const N: usize> TLBSerialize for NumRepr<u32, N> {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder.store_u32(N, self.0)
    }
}

impl<const N: usize> TLBSerialize for NumRepr<u64, N> {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder.store_u64(N, self.0)
    }
}

impl<const N: usize> TLBSerialize for NumRepr<BigUint, N> {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder.store_uint(N, &self.0)
    }
}

impl TLBSerialize for TonAddress {
    fn store<'a>(
        &'a self,
        builder: &'a mut CellBuilder,
    ) -> Result<&'a mut CellBuilder, TonCellError> {
        builder.store_address(self)
    }
}
