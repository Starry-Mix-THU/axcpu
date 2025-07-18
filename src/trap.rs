//! Trap handling.

use memory_addr::VirtAddr;

pub use crate::TrapFrame;
pub use linkme::distributed_slice as def_trap_handler;
pub use linkme::distributed_slice as register_trap_handler;
pub use page_table_entry::MappingFlags as PageFaultFlags;

/// A slice of IRQ handler functions.
#[def_trap_handler]
pub static IRQ: [fn(usize) -> bool];

/// A slice of page fault handler functions.
#[def_trap_handler]
pub static PAGE_FAULT: [fn(VirtAddr, PageFaultFlags, bool) -> bool];

/// A slice of syscall handler functions.
#[cfg(feature = "uspace")]
#[cfg_attr(docsrs, doc(cfg(feature = "uspace")))]
#[def_trap_handler]
pub static SYSCALL: [fn(&mut TrapFrame, usize) -> isize];

/// A slice of callbacks to be invoked before a trap.
#[linkme::distributed_slice]
pub static PRE_TRAP: [fn(&mut TrapFrame, bool)];

/// A slice of callbacks to be invoked after a trap.
#[linkme::distributed_slice]
pub static POST_TRAP: [fn(&mut TrapFrame, bool)];

#[allow(unused_macros)]
macro_rules! handle_trap {
    ($trap:ident, $($args:tt)*) => {{
        let mut iter = $crate::trap::$trap.iter();
        if let Some(func) = iter.next() {
            if iter.next().is_some() {
                warn!("Multiple handlers for trap {} are not currently supported", stringify!($trap));
            }
            func($($args)*)
        } else {
            warn!("No registered handler for trap {}", stringify!($trap));
            false
        }
    }}
}

#[unsafe(no_mangle)]
pub(crate) fn pre_trap_callback(tf: &mut TrapFrame, from_user: bool) {
    for cb in crate::trap::PRE_TRAP.iter() {
        cb(tf, from_user);
    }
}

#[unsafe(no_mangle)]
pub(crate) fn post_trap_callback(tf: &mut TrapFrame, from_user: bool) {
    for cb in crate::trap::POST_TRAP.iter() {
        cb(tf, from_user);
    }
}

/// Call the external syscall handler.
#[cfg(feature = "uspace")]
pub(crate) fn handle_syscall(tf: &mut TrapFrame, syscall_num: usize) -> isize {
    SYSCALL[0](tf, syscall_num)
}
