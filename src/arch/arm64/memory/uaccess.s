
__uacess_begin:
// Copy the type 'T: UserCopyable' from userspace.
//
// Arguments:
//
// x0: src pointer
// x1: dst pointer
// x2: current offset
// x3: total size
//
// Returns:
// x0: status (0 = copy complete, 1 = fault error, 2 = deferred fault)
// x1: work pointer (future)
// x2: current offset
    .globl __do_copy_from_user
    .type __do_copy_from_user, @function
__do_copy_from_user:
    cmp     x2, x3
    beq     1f
    ldrb    w4, [x0, x2]
    strb    w4, [x1, x2]
    add     x2, x2, #1
    b       __do_copy_from_user

// Copy bytes userspace, halting when encountering a NULL byte, or reaching the
// end of the buffer..
//
// Arguments:
//
// x0: src pointer
// x1: dst pointer
// x2: current offset
// x3: dst buffer size
//
// Returns:
// x0: status (0 = copy complete, 1 = fault error, 2 = deferred fault)
// x1: work pointer (future)
// x2: bytes copied (excluding the NULL byte)
    .globl __do_copy_from_user_halt_nul
    .type __do_copy_from_user_halt_nul, @function
__do_copy_from_user_halt_nul:
    cmp     x2, x3
    beq     1f
    ldrb    w4, [x0, x2]
    strb    w4, [x1, x2]
    cmp     w4, #0
    beq     1f
    add     x2, x2, #1
    b       __do_copy_from_user_halt_nul


// Copy the type 'T: UserCopyable' to userspace.
//
// Arguments:
//
// x0: src pointer
// x1: dst pointer
// x2: current offset
// x3: total size
//
// Returns:
// x0: status (0 = copy complete, 1 = fault error, 2 = deferred fault)
// x1: work pointer (future)
// x2: current offset
    .globl __do_copy_to_user
    .type __do_copy_to_user, @function
__do_copy_to_user:
    cmp     x2, x3
    beq     1f
    ldrb    w4, [x0, x2]
    strb    w4, [x1, x2]
    add     x2, x2, #1
    b       __do_copy_to_user

1:  mov     x0, #0
__uacess_end:
fixup:
    ret

    .section .exception_fixups
    .global __UACCESS_FIXUP
__UACCESS_FIXUP:
    .quad   __uacess_begin
    .quad   __uacess_end
    .quad   fixup
