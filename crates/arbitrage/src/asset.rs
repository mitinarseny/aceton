use tlb_ton::MsgAddress;

pub enum Asset {
    Native,
    Jetton(MsgAddress),
}
