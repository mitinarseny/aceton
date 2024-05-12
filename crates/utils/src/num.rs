use std::{
    borrow::Cow,
    fmt::Display,
    marker::PhantomData,
    ops::{Div, DivAssign},
    str::FromStr,
};

use num::{pow::Pow, rational::Ratio, BigUint, FromPrimitive, Integer, Num};
use serde::{
    de::{self, Expected, Unexpected},
    Deserialize, Deserializer,
};
use serde_with::{DeserializeAs, DisplayFromStr, Same};

pub struct Percent<T = Same>(PhantomData<T>);

impl<'de, T, As> DeserializeAs<'de, T> for Percent<As>
where
    As: DeserializeAs<'de, T>,
    T: From<u8> + Div<Output = T>,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: T = As::deserialize_as(deserializer)?;
        Ok(v / T::from(100u8))
    }
}

pub struct DecimalFloatStrAsRatio;

impl<'de, T> DeserializeAs<'de, Ratio<T>> for DecimalFloatStrAsRatio
where
    T: Integer + Clone + From<u8> + Pow<usize, Output = T>,
    <T as Num>::FromStrRadixErr: Display,
{
    fn deserialize_as<D>(deserializer: D) -> Result<Ratio<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = Cow::<&str>::deserialize(deserializer)?;
        if let Some((int, fract)) = s.split_once('.') {
            Ok(
                Ratio::from_integer(T::from_str_radix(int, 10).map_err(de::Error::custom)?)
                    + Ratio::new(
                        T::from_str_radix(fract, 10).map_err(de::Error::custom)?,
                        T::from(10).pow(fract.len()),
                    ),
            )
        } else {
            T::from_str_radix(&s, 10)
                .map(Ratio::from_integer)
                .map_err(de::Error::custom)
        }
    }
}

// pub struct PercentAsRatio;

// impl<'de> DeserializeAs<'de, Ratio<BigUint>> for PercentAsRatio {
//     fn deserialize_as<D>(deserializer: D) -> Result<Ratio<BigUint>, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         DisplayFromStr::deserialize_as(deserializer)?;
//     }
// }
