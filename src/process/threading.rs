use crate::sched::current_task;
use libkernel::{error::Result, memory::address::VA};

pub async fn sys_set_tid_address(_tidptr: VA) -> Result<usize> {
    let tid = current_task().tid;

    // TODO: implement threading and this system call properly. For now, we just
    // return the PID as the thread id.
    Ok(tid.value() as _)
}
