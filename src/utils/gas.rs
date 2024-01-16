use impl_tools::autoimpl;

#[autoimpl(Deref using self.inner)]
#[autoimpl(DerefMut using self.inner)]
pub struct WithGasUsed<T> {
    pub gas_used: u64,
    pub inner: T,
}

impl<T> WithGasUsed<T> {
    pub fn new(gas_used: u64, value: T) -> Self {
        Self {
            gas_used,
            inner: value,
        }
    }
}
