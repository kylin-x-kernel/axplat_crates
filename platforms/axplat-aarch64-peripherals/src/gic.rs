// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 The axplat_crates Authors.
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSE for license details.
// 
// This file has been modified by KylinSoft on 2025.

//! ARM Generic Interrupt Controller (GIC).

#[cfg(feature = "gicv2")]
use arm_gic_driver::v2::*;
#[cfg(feature = "gicv3")]
use arm_gic_driver::v3::*;

use axplat::irq::{HandlerTable, IpiTarget, IrqHandler};
use core::arch::asm;
use aarch64_cpu::registers::{DAIF, Readable};
use kspin::SpinNoIrq;
use lazyinit::LazyInit;

/// The maximum number of IRQs.
const MAX_IRQ_COUNT: usize = 1024;

static GIC: LazyInit<SpinNoIrq<Gic>> = LazyInit::new();

static TRAP_OP: LazyInit<TrapOp> = LazyInit::new();

static IRQ_HANDLER_TABLE: HandlerTable<MAX_IRQ_COUNT> = HandlerTable::new();

/// set trigger type of given IRQ
pub fn set_trigger(irq_num: usize, edge: bool) {
    trace!("GIC set trigger: {} {}", irq_num, edge);
    let intid = unsafe { IntId::raw(irq_num as u32) };
    let cfg = if edge {
        Trigger::Edge
    } else {
        Trigger::Level
    };
    GIC.lock().set_cfg(intid, cfg);
}

/// Enables or disables the given IRQ.
pub fn set_enable(irq: usize, enabled: bool) {
    trace!("GIC set enable: {irq} {enabled}");
    let intid = unsafe { IntId::raw(irq as u32) };
    let gic = GIC.lock();
    gic.set_irq_enable(intid, enabled);
    if !intid.is_private() {
        gic.set_cfg(intid, Trigger::Edge);
    }
}

/// Registers an IRQ handler for the given IRQ.
///
/// It also enables the IRQ if the registration succeeds. It returns `false`
/// if the registration failed.
pub fn register_handler(irq: usize, handler: IrqHandler) -> bool {
    if IRQ_HANDLER_TABLE.register_handler(irq, handler) {
        trace!("register handler IRQ {irq}");
        set_enable(irq, true);
        return true;
    }
    warn!("register handler for IRQ {irq} failed");
    false
}

/// Unregisters the IRQ handler for the given IRQ.
///
/// It also disables the IRQ if the unregistration succeeds. It returns the
/// existing handler if it is registered, `None` otherwise.
pub fn unregister_handler(irq: usize) -> Option<IrqHandler> {
    trace!("unregister handler IRQ {irq}");
    set_enable(irq, false);
    IRQ_HANDLER_TABLE.unregister_handler(irq)
}

/// Sets the priority for a specific interrupt request (IRQ).
///
/// This function configures the priority level for the given IRQ number. Lower
/// numerical values indicate higher priority. The priority value must be within
/// the valid range supported by the interrupt controller.
pub fn set_priority(irq: usize, priority: u8) {
    let intid = unsafe { IntId::raw(irq as u32) };
    let gic = GIC.lock();
    gic.set_priority(intid, priority);
}

/// Sets the priority mask for the CPU interface.
///
/// This function configures the priority mask register (PMR) which determines
/// the minimum priority level that can interrupt the processor. Interrupts with
/// priority lower than this mask will be ignored. This is useful for implementing
/// priority-based interrupt masking.
pub fn set_priority_mask(priority: u8) {
    let gic = GIC.lock();
    gic.cpu_interface().set_priority_mask(priority);
}   

/// Gets the current priority mask of the CPU interface.
///
/// This function reads the priority mask register (PMR) of the GIC CPU interface.
/// The PMR defines the minimum interrupt priority level that is allowed to be
/// signaled to the processor. Interrupts with a priority value numerically
/// greater than the PMR are masked and will not be delivered to the CPU.
pub fn get_priority_mask() -> u8 {
    let gic = GIC.lock();
    gic.cpu_interface().get_priority_mask()
}   

/// Handles the IRQ.
///
/// It is called by the common interrupt handler. It should look up in the
/// IRQ handler table and calls the corresponding handler. If necessary, it
/// also acknowledges the interrupt controller after handling.
#[cfg(feature = "gicv2")]
pub fn handle_irq(_unused: usize) -> Option<usize> {
    let ack = TRAP_OP.ack();

    if ack.is_special() {
        return None;
    }

    let irq = match ack {
        Ack::Other(intid) => intid,
        Ack::SGI { intid, cpu_id: _ } => intid,
    }
    .to_u32() as usize;

    trace!("IRQ: {ack:?}");

    if !IRQ_HANDLER_TABLE.handle(irq) {
        debug!("Unhandled IRQ {ack:?}");
    }

    TRAP_OP.eoi(ack);
    if TRAP_OP.eoi_mode_ns() {
        TRAP_OP.dir(ack);
    }

    Some(irq)
}

#[cfg(feature = "gicv3")]
pub fn handle_irq(_unused: usize) -> Option<usize> {
    let ack = TRAP_OP.ack1();
    if ack.is_special() {
        return None;
    }

    trace!("Handling IRQ: {ack:?}");

    if !IRQ_HANDLER_TABLE.handle(ack.to_u32() as _) {
        warn!("Unhandled IRQ {:?}", ack);
    }

    TRAP_OP.eoi1(ack);
    if TRAP_OP.eoi_mode() {
        TRAP_OP.dir(ack);
    }

    Some(ack.to_u32() as usize)
}

/// Initializes GIC
#[cfg(feature = "gicv2")]
pub fn init_gic(gicd_base: axplat::mem::VirtAddr, gicc_base: axplat::mem::VirtAddr) {
    info!("Initialize GICv2...");
    let gicd_base = VirtAddr::new(gicd_base.into());
    let gicc_base = VirtAddr::new(gicc_base.into());

    let mut gic = unsafe { Gic::new(gicd_base, gicc_base, None) };
    gic.init();

    GIC.init_once(SpinNoIrq::new(gic));
    let cpu = GIC.lock().cpu_interface();
    TRAP_OP.init_once(cpu.trap_operations());
}

/// Initializes GIC
#[cfg(feature = "gicv3")]
pub fn init_gic(gicd_base: axplat::mem::VirtAddr, gicr_base: axplat::mem::VirtAddr) {
    info!("Initialize GICv3...");
    let gicd_base = VirtAddr::new(gicd_base.into());
    let gicr_base = VirtAddr::new(gicr_base.into());

    let mut gic = unsafe { Gic::new(gicd_base, gicr_base) };
    gic.init();
    GIC.init_once(SpinNoIrq::new(gic));
    let cpu = GIC.lock().cpu_interface();
    TRAP_OP.init_once(cpu.trap_operations());
}

/// Initializes GICC (for all CPUs).
///
/// It must be called after [`init_gic`].
#[cfg(feature = "gicv2")]
pub fn init_gicc() {
    debug!("Initialize GIC CPU Interface...");
    let mut cpu = GIC.lock().cpu_interface();
    cpu.init_current_cpu();
    cpu.set_eoi_mode_ns(false);
}

/// Initializes GICR (for all CPUs).
#[cfg(feature = "gicv3")]
pub fn init_gicr() {
    debug!("Initialize GIC CPU Interface...");
    let mut cpu = GIC.lock().cpu_interface();
    let _ = cpu.init_current_cpu();
    cpu.set_eoi_mode(false);
}

/// Sends an inter-processor interrupt (IPI) to the specified target CPU or all CPUs.
#[cfg(feature = "gicv2")]
pub fn send_ipi(irq_num: usize, target: IpiTarget) {
    match target {
        IpiTarget::Current { cpu_id: _ } => {
            GIC.lock()
                .send_sgi(IntId::sgi(irq_num as u32), SGITarget::Current);
        }
        IpiTarget::Other { cpu_id } => {
            let target_list = TargetList::new(&mut [cpu_id].into_iter());
            GIC.lock().send_sgi(
                IntId::sgi(irq_num as u32),
                SGITarget::TargetList(target_list),
            );
        }
        IpiTarget::AllExceptCurrent {
            cpu_id: _,
            cpu_num: _,
        } => {
            GIC.lock()
                .send_sgi(IntId::sgi(irq_num as u32), SGITarget::AllOther);
        }
    }
}

#[cfg(feature = "gicv3")]
pub fn send_ipi(irq_num: usize, target: IpiTarget) {
    match target {
        IpiTarget::Current { cpu_id: _ } => {
            GIC.lock().cpu_interface()
                .send_sgi(IntId::sgi(irq_num as u32), SGITarget::current());
            }
        IpiTarget::Other { cpu_id } => {
            let affinity = Affinity::from_mpidr(cpu_id as u64);
            let target_list = TargetList::new([affinity]);
            GIC.lock().cpu_interface().send_sgi(
                IntId::sgi(irq_num as u32),
                SGITarget::List(target_list),
            );
        }
        IpiTarget::AllExceptCurrent {
            cpu_id: _,
            cpu_num: _,
        } => {
            GIC.lock().cpu_interface()
                .send_sgi(IntId::sgi(irq_num as u32), SGITarget::All);
        }
    }
}

/// Allows the current CPU to respond to interrupts.
///
/// In AArch64, it unmasks IRQs by clearing the I bit in the `DAIF` register.
#[cfg(not(feature = "pmr"))]
#[inline]
pub fn enable_irqs() {
    // Default implementation: via DAIF register
    unsafe { asm!("msr daifclr, #2") };
}

/// Makes the current CPU ignore interrupts.
///
/// In AArch64, it masks IRQs by setting the I bit in the `DAIF` register.
#[cfg(not(feature = "pmr"))]
#[inline]
pub fn disable_irqs() {
    // Default implementation: via DAIF register
    unsafe { asm!("msr daifset, #2") };
}

/// Returns whether the current CPU is allowed to respond to interrupts.
///
/// In AArch64, it checks the I bit in the `DAIF` register.
#[cfg(not(feature = "pmr"))]
#[inline]
pub fn irqs_enabled() -> bool {
    !DAIF.matches_all(DAIF::I::Masked)
}

#[cfg(not(feature = "pmr"))]
#[inline]
pub fn local_irq_save_and_disable() -> usize {
    let flags: usize;
    // save `DAIF` flags
    unsafe { asm!("mrs {}, daif", out(reg) flags) };
    disable_irqs();
    flags
}

#[cfg(not(feature = "pmr"))]
#[inline]
pub fn local_irq_restore(flags: usize) {
    unsafe { asm!("msr daif, {}", in(reg) flags) };
}

/// Allows the current CPU to respond to interrupts.
///
/// When the GIC CPU interface is not initialized yet (early boot),
/// fall back to DAIF-based IRQ unmasking.
/// After GIC initialization, IRQ masking/unmasking is controlled
/// via the GIC priority mask (PMR).
#[cfg(feature = "pmr")]
#[inline]
pub fn enable_irqs() {
    if !GIC.is_inited() {
        // Early boot: GIC CPU interface not ready, use DAIF directly
        unsafe { asm!("msr daifclr, #2") };
    } else {
        // Normal path: unmask all IRQ priorities via GIC PMR
        set_priority_mask(0xff);
        unsafe { asm!("msr daifclr, #2") };
    }
}

/// Makes the current CPU ignore interrupts.
///
/// During early boot, IRQs are masked using DAIF.
/// Once the GIC CPU interface is initialized, IRQ masking is done
/// via the GIC priority mask (PMR).
#[cfg(feature = "pmr")]
#[inline]
pub fn disable_irqs() {
    if !GIC.is_inited() {
        // Early boot: mask IRQs via DAIF
        unsafe { asm!("msr daifset, #2") };
    } else {
        // Normal path: raise GIC priority mask to block IRQs
        set_priority_mask(0x80);
        unsafe { asm!("msr daifclr, #2") };
    }
}

/// Returns whether the current CPU is allowed to respond to interrupts.
///
/// In early boot, this is determined solely by the DAIF register.
/// After GIC initialization, both DAIF and the GIC priority mask
/// are considered.
#[cfg(feature = "pmr")]
#[inline]
pub fn irqs_enabled() -> bool {
    if !GIC.is_inited() {
        !DAIF.matches_all(DAIF::I::Masked)
    } else {
        !DAIF.matches_all(DAIF::I::Masked) && get_priority_mask() > 0xa0
    }
}


/// Save the current interrupt state and disable IRQs.
///
/// This function may be called during early boot, before the GIC CPU interface
/// is initialized. In that case, it falls back to manipulating the DAIF register
/// directly to mask IRQs.
///
/// After the GIC has been initialized, IRQ masking is performed via the GIC
/// priority mask (PMR) instead.
#[cfg(feature = "pmr")]
#[inline]
pub fn local_irq_save_and_disable() -> usize {
    if GIC.is_inited() {
        let pmr = get_priority_mask() as usize;
        disable_irqs();
        pmr
    } else {
        let flags: usize;
        // Save DAIF and mask IRQs via the I bit (early boot path)
        unsafe { asm!("mrs {}, daif; msr daifset, #2", out(reg) flags) };
        flags
    }
}

/// Restore the interrupt state saved by [`local_irq_save_and_disable`].
///
/// If the GIC has already been initialized, the saved value is interpreted as a
/// GIC priority mask and restored via the PMR. Otherwise, the saved DAIF value
/// is written back directly (early boot path).
#[cfg(feature = "pmr")]
#[inline]
pub fn local_irq_restore(flags: usize) {
    if GIC.is_inited() {
        set_priority_mask(flags as u8);
    } else {
        unsafe { asm!("msr daif, {}", in(reg) flags) };
    }
}

/// Default implementation of [`axplat::irq::IrqIf`] using the GIC.
#[macro_export]
macro_rules! irq_if_impl {
    ($name:ident) => {
        struct $name;

        #[impl_plat_interface]
        impl axplat::irq::IrqIf for $name {
            /// Enables or disables the given IRQ.
            fn set_enable(irq: usize, enabled: bool) {
                $crate::gic::set_enable(irq, enabled);
            }

            /// Registers an IRQ handler for the given IRQ.
            ///
            /// It also enables the IRQ if the registration succeeds. It returns `false`
            /// if the registration failed.
            fn register(irq: usize, handler: axplat::irq::IrqHandler) -> bool {
                $crate::gic::register_handler(irq, handler)
            }

            /// Unregisters the IRQ handler for the given IRQ.
            ///
            /// It also disables the IRQ if the unregistration succeeds. It returns the
            /// existing handler if it is registered, `None` otherwise.
            fn unregister(irq: usize) -> Option<axplat::irq::IrqHandler> {
                $crate::gic::unregister_handler(irq)
            }

            /// Handles the IRQ.
            ///
            /// It is called by the common interrupt handler. It should look up in the
            /// IRQ handler table and calls the corresponding handler. If necessary, it
            /// also acknowledges the interrupt controller after handling.
            fn handle(irq: usize) -> Option<usize> {
                $crate::gic::handle_irq(irq)
            }

            /// Sends an inter-processor interrupt (IPI) to the specified target CPU or all CPUs.
            fn send_ipi(irq_num: usize, target: axplat::irq::IpiTarget) {
                $crate::gic::send_ipi(irq_num, target);
            }

            /// Sets the priority for a specific interrupt request (IRQ).
            fn set_priority(irq: usize, priority: u8) {
                $crate::gic::set_priority(irq, priority);
            }

            /// Save irq status and disable
            fn local_irq_save_and_disable() -> usize {
                $crate::gic::local_irq_save_and_disable()
            }

            /// Restore irq status
            fn local_irq_restore(flag: usize) {
                $crate::gic::local_irq_restore(flag);
            }

            /// Allows the current CPU to respond to interrupts.
            fn enable_irqs(){
                $crate::gic::enable_irqs();
            }

            /// Makes the current CPU ignore interrupts.
            fn disable_irqs(){
                $crate::gic::disable_irqs();
            }

            /// Returns whether the current CPU is allowed to respond to interrupts.
            fn irqs_enabled() -> bool {
                $crate::gic::irqs_enabled()
            }
        }
    };
}
