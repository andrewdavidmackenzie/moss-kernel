#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct CharDevDescriptor {
    pub major: u64,
    pub minor: u64,
}
