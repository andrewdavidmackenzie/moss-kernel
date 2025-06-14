use core::fmt::Display;

use alloc::{boxed::Box, sync::Arc};
use libkernel::error::Result;

use super::{Driver, DriverManager};

bitflags::bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct FdtFlags: u32 {
        const ACTIVE_CONSOLE = 1;
    }
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DeviceMatchType {
    FdtCompatible(&'static str),
}

#[derive(Clone)]
pub enum DeviceDescriptor {
    Fdt(fdt_parser::Node<'static>, FdtFlags),
}

impl Display for DeviceDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DeviceDescriptor::Fdt(node, _) => f.write_str(node.name),
        }
    }
}

pub type ProbeFn =
    Box<dyn Fn(&mut DriverManager, DeviceDescriptor) -> Result<Arc<dyn Driver>> + Send>;
