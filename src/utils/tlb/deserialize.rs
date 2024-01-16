use tonlib::cell::{Cell, CellParser, TonCellError};

pub trait TLBDeserialize: Sized {
    fn load(parser: &mut CellParser) -> Result<Self, TonCellError>;

    fn parse_fully(cell: &Cell) -> Result<Self, TonCellError> {
        cell.parse_fully(Self::load)
    }
}

pub trait CellParserExt {
    fn load<T>(&mut self) -> Result<T, TonCellError>
    where
        T: TLBDeserialize;
}

impl<'a> CellParserExt for CellParser<'a> {
    fn load<T>(&mut self) -> Result<T, TonCellError>
    where
        T: TLBDeserialize,
    {
        T::load(self)
    }
}

pub trait CellExt {
    fn parse_to<T>(&self) -> Result<T, TonCellError>
    where
        T: TLBDeserialize;
    fn parse_to_fully<T>(&self) -> Result<T, TonCellError>
    where
        T: TLBDeserialize;
}

impl CellExt for Cell {
    fn parse_to<T>(&self) -> Result<T, TonCellError>
    where
        T: TLBDeserialize,
    {
        self.parse(T::load)
    }

    fn parse_to_fully<T>(&self) -> Result<T, TonCellError>
    where
        T: TLBDeserialize,
    {
        self.parse_fully(T::load)
    }
}
