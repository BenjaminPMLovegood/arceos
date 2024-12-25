//! Interrupt management.

use handler_table::HandlerTable;

use crate::platform::irq::{dispatch_irq, MAX_IRQ_COUNT};
use crate::trap::{register_trap_handler, IRQ};

pub use crate::platform::irq::{register_handler, set_enable};

#[cfg(target_arch = "aarch64")]
pub use crate::platform::irq::{fetch_irq, GicInterface};

/// The type if an IRQ handler.
pub type IrqHandler = handler_table::Handler;

static IRQ_HANDLER_TABLE: HandlerTable<MAX_IRQ_COUNT> = HandlerTable::new();

/// Platform-independent IRQ dispatching.
#[allow(dead_code)]
pub(crate) fn dispatch_irq_common(irq_num: usize) {
    trace!("IRQ {}", irq_num);
    if !IRQ_HANDLER_TABLE.handle(irq_num) {
        warn!("Unhandled IRQ {}", irq_num);
    }
}

/// Platform-independent IRQ handler registration.
///
/// It also enables the IRQ if the registration succeeds. It returns `false` if
/// the registration failed.
#[allow(dead_code)]
pub(crate) fn register_handler_common(irq_num: usize, handler: IrqHandler) -> bool {
    if irq_num < MAX_IRQ_COUNT && IRQ_HANDLER_TABLE.register_handler(irq_num, handler) {
        set_enable(irq_num, true);
        return true;
    }
    warn!("register handler for IRQ {} failed", irq_num);
    false
}

/// Core IRQ handling routine, registered at `axhal::trap::IRQ`,
/// which dispatches IRQs to registered handlers.
///
/// Note: this function is denoted as public here because it'll be called by the
/// hypervisor for hypervisor reserved IRQ handling.
#[register_trap_handler(IRQ)]
pub fn handler_irq(irq_num: usize) -> bool {
    let guard = kernel_guard::NoPreempt::new();
    dispatch_irq(irq_num);
    drop(guard); // rescheduling may occur when preemption is re-enabled.
    true
}

/// A trait for interrupt controller drivers that handle hardware interrupts.
///
/// This trait defines the basic operations that any interrupt controller should implement,
/// including clearing, fetching, enabling/disabling interrupts, and inter-processor interrupts (IPI).
///
/// # Methods
///
/// * `clear_current_irq` - Clears the currently active interrupt
/// * `clear_irq` - Clears all pending interrupts
/// * `fetch_irq` - Gets the next pending interrupt number if any
/// * `finish_irq` - Completes handling of current interrupt
/// * `enable_irq` - Enables or disables a specific interrupt
/// * `register_irq` - Registers an interrupt number for handling
/// * `inject_irq` - Injects/triggers a specific interrupt
/// * `send_ipi` - Sends an inter-processor interrupt to another CPU
///
/// # Safety
///
/// Implementations must ensure proper hardware synchronization and interrupt handling.
pub trait IrqController {
    fn clear_current_irq(for_hypervisor: bool);
    fn clear_irq();
    fn fetch_irq() -> Option<usize>;
    fn finish_irq();
    fn enable_irq(irq_num: usize, enable: bool);
    fn register_irq(irq_num: usize) -> bool;
    fn inject_irq(irq_num: usize);
    fn send_ipi(cpu_id: usize, ipi_id: usize);
}
