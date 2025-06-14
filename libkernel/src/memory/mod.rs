pub mod address;
pub mod kbuf;
pub mod page;
pub mod page_alloc;
pub mod permissions;
pub mod pg_offset;
pub mod proc_vm;
pub mod region;
pub mod smalloc;

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SHIFT: usize = PAGE_SIZE.trailing_zeros() as usize;
pub const PAGE_MASK: usize = PAGE_SIZE - 1;
