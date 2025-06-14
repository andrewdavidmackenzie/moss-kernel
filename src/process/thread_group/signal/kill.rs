use crate::{
    process::{Tid, thread_group::pid::PidT},
    sched::current_task,
};
use libkernel::error::{KernelError, Result};

use super::{SigId, uaccess::UserSigId};

pub fn sys_kill(_pid: PidT, _signal: UserSigId) -> Result<usize> {
    // let target_tg = Tgid(pid as _);

    // let signal: SigId = signal.try_into()?;

    todo!();
}

pub fn sys_tkill(tid: PidT, signal: UserSigId) -> Result<usize> {
    let target_tid = Tid(tid as _);
    let current_task = current_task();

    let signal: SigId = signal.try_into()?;

    // The fast-path case.
    if current_task.tid == target_tid {
        current_task
            .process
            .signals
            .lock_save_irq()
            .set_pending(signal);
    } else {
        let task = current_task
            .process
            .threads
            .lock_save_irq()
            .get(&target_tid)
            .and_then(|t| t.upgrade())
            .ok_or(KernelError::NoProcess)?;

        task.process.signals.lock_save_irq().set_pending(signal);
    }

    Ok(0)
}
