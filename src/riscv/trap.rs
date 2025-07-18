use riscv::interrupt::supervisor::{Exception as E, Interrupt as I};
use riscv::interrupt::Trap;
#[cfg(feature = "fp-simd")]
use riscv::register::sstatus;
use riscv::register::{scause, stval};

use super::TrapFrame;
use crate::trap::PageFaultFlags;

core::arch::global_asm!(
    include_asm_macros!(),
    include_str!("trap.S"),
    trapframe_size = const core::mem::size_of::<TrapFrame>(),
);

fn handle_breakpoint(sepc: &mut usize) {
    debug!("Exception(Breakpoint) @ {sepc:#x} ");
    *sepc += 2
}

fn handle_page_fault(tf: &TrapFrame, mut access_flags: PageFaultFlags, is_user: bool) {
    if is_user {
        access_flags |= PageFaultFlags::USER;
    }
    let vaddr = va!(stval::read());
    if !handle_trap!(PAGE_FAULT, vaddr, access_flags, is_user) {
        panic!(
            "Unhandled {} Page Fault @ {:#x}, fault_vaddr={:#x} ({:?}):\n{:#x?}\n{}",
            if is_user { "User" } else { "Supervisor" },
            tf.sepc,
            vaddr,
            access_flags,
            tf,
            tf.backtrace()
        );
    }
}

#[unsafe(no_mangle)]
fn riscv_trap_handler(tf: &mut TrapFrame, from_user: bool) {
    let scause = scause::read();
    if let Ok(cause) = scause.cause().try_into::<I, E>() {
        crate::trap::pre_trap_callback(tf, from_user);
        match cause {
            #[cfg(feature = "uspace")]
            Trap::Exception(E::UserEnvCall) => {
                tf.sepc += 4;
                tf.regs.a0 = crate::trap::handle_syscall(tf, tf.regs.a7) as usize;
            }
            Trap::Exception(E::LoadPageFault) => {
                handle_page_fault(tf, PageFaultFlags::READ, from_user)
            }
            Trap::Exception(E::StorePageFault) => {
                handle_page_fault(tf, PageFaultFlags::WRITE, from_user)
            }
            Trap::Exception(E::InstructionPageFault) => {
                handle_page_fault(tf, PageFaultFlags::EXECUTE, from_user)
            }
            Trap::Exception(E::Breakpoint) => handle_breakpoint(&mut tf.sepc),
            Trap::Interrupt(_) => {
                handle_trap!(IRQ, scause.bits());
            }
            _ => {
                panic!(
                    "Unhandled trap {:?} @ {:#x}, stval={:#x}:\n{:#x?}\n{}",
                    cause,
                    tf.sepc,
                    stval::read(),
                    tf,
                    tf.backtrace()
                );
            }
        }
        crate::trap::post_trap_callback(tf, from_user);
    } else {
        panic!(
            "Unknown trap {:#x?} @ {:#x}:\n{:#x?}\n{}",
            scause.cause(),
            tf.sepc,
            tf,
            tf.backtrace()
        );
    }

    // Update tf.sstatus to preserve current hardware FS state
    // This replaces the assembly-level FS handling workaround
    #[cfg(feature = "fp-simd")]
    tf.sstatus.set_fs(sstatus::read().fs());
}
