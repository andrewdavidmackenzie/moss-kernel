#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Uid(u32);

impl Uid {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn is_root(self) -> bool {
        self.0 == 0
    }

    pub fn new_root() -> Self {
        Self(0)
    }
}

impl From<Uid> for u32 {
    fn from(value: Uid) -> Self {
        value.0
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Gid(u32);

impl Gid {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn new_root_group() -> Self {
        Self(0)
    }
}

impl From<Gid> for u32 {
    fn from(value: Gid) -> Self {
        value.0
    }
}
