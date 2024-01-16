use strum::Display;

pub mod dedust;
pub mod stonfi;

#[derive(Display)]
#[strum(serialize_all = "snake_case")]
pub enum DEX {
    Stonfi,
    Dedust,
}
