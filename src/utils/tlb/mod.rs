mod deserialize;
mod serialize;

use tonlib::cell::Cell;

pub use self::{deserialize::*, serialize::*};

use std::sync::Arc;

pub struct RefCell(pub Arc<Cell>);

impl From<Arc<Cell>> for RefCell {
    fn from(value: Arc<Cell>) -> Self {
        Self(value)
    }
}

impl From<Cell> for RefCell {
    fn from(value: Cell) -> Self {
        Self(value.into())
    }
}

pub struct NumRepr<T, const N: usize>(pub T);
