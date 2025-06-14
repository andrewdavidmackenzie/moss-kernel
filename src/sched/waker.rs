use crate::process::{TASK_LIST, TaskDescriptor, TaskState};
use core::task::{RawWaker, RawWakerVTable, Waker};

unsafe fn clone_waker(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}

/// Wakes the task. This consumes the waker.
unsafe fn wake_waker(data: *const ()) {
    let desc = TaskDescriptor::from_ptr(data);

    if let Some(proc) = TASK_LIST.lock_save_irq().get(&desc)
        && let Some(proc) = proc.upgrade()
    {
        let mut state = proc.lock_save_irq();
        if *state == TaskState::Sleeping {
            *state = TaskState::Runnable;
        }
    }
}

unsafe fn drop_waker(_data: *const ()) {
    // There is nothing to do.
}

static VTABLE: RawWakerVTable =
    RawWakerVTable::new(clone_waker, wake_waker, wake_waker, drop_waker);

/// Creates a `Waker` for a given `Pid`.
pub fn create_waker(desc: TaskDescriptor) -> Waker {
    let raw_waker = RawWaker::new(desc.to_ptr(), &VTABLE);

    // SAFETY: We have correctly implemented the VTable functions.
    unsafe { Waker::from_raw(raw_waker) }
}
