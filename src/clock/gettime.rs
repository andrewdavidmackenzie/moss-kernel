use libkernel::{
    error::{KernelError, Result},
    memory::address::TUA,
};

use crate::{drivers::timer::uptime, memory::uaccess::copy_to_user};

use super::{realtime::date, timespec::TimeSpec};

pub type ClockId = i32;

const CLOCK_MONOTONIC: ClockId = 0;
const CLOCK_REALTIME: ClockId = 1;

pub async fn sys_clock_gettime(clockid: ClockId, time_spec: TUA<TimeSpec>) -> Result<usize> {
    let time = match clockid {
        CLOCK_MONOTONIC => uptime(),
        CLOCK_REALTIME => date(),
        _ => return Err(KernelError::InvalidValue),
    };

    copy_to_user(time_spec, time.into()).await?;

    Ok(0)
}
