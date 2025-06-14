use crate::{fs::VFS, memory::uaccess::cstr::UserCStr, process::fd_table::Fd, sched::current_task};
use core::ffi::c_char;
use libkernel::{
    error::Result,
    fs::{OpenFlags, path::Path},
    memory::address::TUA,
};

use super::resolve_at_start_node;

pub async fn sys_openat(
    dirfd: Fd,
    path: TUA<c_char>,
    flags: u32,
    _mode: u16, // Permissions for file creation
) -> Result<usize> {
    let mut buf = [0; 1024];

    let flags = OpenFlags::from_bits_truncate(flags);
    let path = Path::new(UserCStr::from_ptr(path).copy_from_user(&mut buf).await?);
    let start_node = resolve_at_start_node(dirfd, path).await?;

    let file = VFS.open(path, flags, start_node).await?;

    let fd = current_task().fd_table.lock_save_irq().insert(file)?;

    Ok(fd.as_raw() as _)
}
