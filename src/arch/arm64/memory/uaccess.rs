use core::{
    arch::{asm, global_asm},
    future::Future,
    mem::transmute,
    pin::Pin,
    task::{Context, Poll},
};

use alloc::boxed::Box;
use libkernel::{
    error::{KernelError, Result},
    memory::address::UA,
};
use log::error;

global_asm!(include_str!("uaccess.s"));

type Fut = dyn Future<Output = Result<()>> + Send;

unsafe impl Send for Arm64CopyFromUser {}
unsafe impl Send for Arm64CopyToUser {}
unsafe impl Send for Arm64CopyStrnFromUser {}

pub const UACESS_ABORT_DENIED: u64 = 1;
pub const UACESS_ABORT_DEFERRED: u64 = 2;

/// A helper function to handle the common polling logic for uaccess operations.
fn poll_uaccess<F>(
    deferred_fault: &mut Option<Pin<Box<Fut>>>,
    bytes_coped: &mut usize,
    cx: &mut Context<'_>,
    mut do_copy: F,
) -> Poll<Result<usize>>
where
    F: FnMut(usize) -> (u64, usize, usize, usize),
{
    // First, if a deferred fault has been set, poll that.
    loop {
        if let Some(mut fut) = deferred_fault.take() {
            match fut.as_mut().poll(cx) {
                Poll::Ready(Err(_)) => return Poll::Ready(Err(KernelError::Fault)),
                Poll::Ready(Ok(())) => {}
                Poll::Pending => {
                    *deferred_fault = Some(fut);
                    return Poll::Pending;
                }
            }
        }

        // Let's move some data. The let bindings here are the return values
        // from the assmebly call.
        let (status, work_ptr, work_vtable, new_bytes_copied) = do_copy(*bytes_coped);

        match status {
            0 => return Poll::Ready(Ok(new_bytes_copied)),
            UACESS_ABORT_DENIED => return Poll::Ready(Err(KernelError::Fault)),
            UACESS_ABORT_DEFERRED => {
                *bytes_coped = new_bytes_copied;
                let ptr: *mut Fut =
                    unsafe { transmute((work_ptr as *mut (), work_vtable as *const ())) };
                *deferred_fault = Some(unsafe { Box::into_pin(Box::from_raw(ptr)) });
            }
            _ => {
                error!("Unknown exit status from fault handler: {status}");
                return Poll::Ready(Err(KernelError::Fault));
            }
        }
    }
}

pub struct Arm64CopyFromUser {
    src: UA,
    dst: *const (),
    len: usize,
    bytes_coped: usize,
    deferred_fault: Option<Pin<Box<Fut>>>,
}

impl Arm64CopyFromUser {
    pub fn new(src: UA, dst: *const (), len: usize) -> Self {
        Self {
            src,
            dst,
            len,
            bytes_coped: 0,
            deferred_fault: None,
        }
    }
}

impl Future for Arm64CopyFromUser {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        poll_uaccess(
            &mut this.deferred_fault,
            &mut this.bytes_coped,
            cx,
            |mut bytes_copied| {
                let mut status: u64;
                let mut work_ptr: usize;
                let mut work_vtable: usize;

                unsafe {
                    asm!(
                        "bl __do_copy_from_user",
                        in("x0") this.src.value(),
                        in("x1") this.dst,
                        inout("x2") bytes_copied,
                        in("x3") this.len,
                        lateout("x0") status,
                        lateout("x1") work_ptr,
                        lateout("x3") work_vtable,
                        // Clobbers
                        out("lr") _, out("x4") _
                    )
                }

                (status, work_ptr, work_vtable, bytes_copied)
            },
        )
        .map(|x| x.map(|_| ()))
    }
}

pub struct Arm64CopyStrnFromUser {
    src: UA,
    dst: *mut u8,
    len: usize,
    bytes_coped: usize,
    deferred_fault: Option<Pin<Box<Fut>>>,
}

impl Arm64CopyStrnFromUser {
    pub fn new(src: UA, dst: *mut u8, len: usize) -> Self {
        Self {
            src,
            dst,
            len,
            bytes_coped: 0,
            deferred_fault: None,
        }
    }
}

impl Future for Arm64CopyStrnFromUser {
    type Output = Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        poll_uaccess(
            &mut this.deferred_fault,
            &mut this.bytes_coped,
            cx,
            |mut bytes_copied| {
                let mut status: u64;
                let mut work_ptr: usize;
                let mut work_vtable: usize;

                unsafe {
                    asm!(
                        "bl __do_copy_from_user_halt_nul",
                        in("x0") this.src.value(),
                        in("x1") this.dst,
                        inout("x2") bytes_copied,
                        in("x3") this.len,
                        lateout("x0") status,
                        lateout("x1") work_ptr,
                        lateout("x3") work_vtable,
                        // Clobbers
                        out("lr") _, out("x4") _
                    )
                }

                (status, work_ptr, work_vtable, bytes_copied)
            },
        )
    }
}

pub struct Arm64CopyToUser {
    src: *const (),
    dst: UA,
    len: usize,
    bytes_coped: usize,
    deferred_fault: Option<Pin<Box<Fut>>>,
}

impl Arm64CopyToUser {
    pub fn new(src: *const (), dst: UA, len: usize) -> Self {
        Self {
            src,
            dst,
            len,
            bytes_coped: 0,
            deferred_fault: None,
        }
    }
}

impl Future for Arm64CopyToUser {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        poll_uaccess(
            &mut this.deferred_fault,
            &mut this.bytes_coped,
            cx,
            |mut bytes_copied| {
                let mut status: u64;
                let mut work_ptr: usize;
                let mut work_vtable: usize;

                unsafe {
                    asm!(
                        "bl __do_copy_to_user",
                        in("x0") this.src,
                        in("x1") this.dst.value(),
                        inout("x2") bytes_copied,
                        in("x3") this.len,
                        lateout("x0") status,
                        lateout("x1") work_ptr,
                        lateout("x3") work_vtable,
                        // Clobbers
                        out("lr") _, out("x4") _
                    )
                }
                (status, work_ptr, work_vtable, bytes_copied)
            },
        )
        .map(|x| x.map(|_| ()))
    }
}
