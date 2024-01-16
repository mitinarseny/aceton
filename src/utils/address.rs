#[macro_export]
macro_rules! ton_address {
    ($($(#[$attr:meta])*
    $ident:ident = $addr:literal);+) => {
        ::lazy_static::lazy_static! {
            $(
            $(#[$attr])*
            static ref $ident: ::tonlib::address::TonAddress = $addr.parse().unwrap();
            )+
        }
    };

    ($($(#[$attr:meta])*
    pub $ident:ident = $addr:literal);+) => {
        ::lazy_static::lazy_static! {
            $(
            $(#[$attr])*
            pub static ref $ident: ::tonlib::address::TonAddress = $addr.parse().unwrap();
            )+
        }
    };

    ($($(#[$attr:meta])*
    pub $($vis:tt)+ $ident:ident = $addr:literal);+) => {
        ::lazy_static::lazy_static! {
            $(
            $(#[$attr])*
            pub $($vis)+ static ref $ident: ::tonlib::address::TonAddress = $addr.parse().unwrap();
            )+
        }
    };
}
