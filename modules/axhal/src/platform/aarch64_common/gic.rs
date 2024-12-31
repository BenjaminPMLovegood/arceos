use crate::{irq::IrqHandler, mem::phys_to_virt};
use arm_gicv2::{
    translate_irq, GicCpuInterface, GicDistributor, GicHypervisorInterface, InterruptType,
};
use kspin::SpinNoIrq;
use memory_addr::PhysAddr;

/// The maximum number of IRQs.
pub const MAX_IRQ_COUNT: usize = 1024;

#[cfg(not(feature = "hv"))]
/// The timer IRQ number.
pub const TIMER_IRQ_NUM: usize = translate_irq(14, InterruptType::PPI).unwrap();

#[cfg(feature = "hv")]
/// Non-secure EL2 Physical Timer irq number.
pub const TIMER_IRQ_NUM: usize = translate_irq(10, InterruptType::PPI).unwrap();

/// The UART IRQ number.
pub const UART_IRQ_NUM: usize = translate_irq(axconfig::UART_IRQ, InterruptType::SPI).unwrap();

const GICD_BASE: PhysAddr = pa!(axconfig::GICD_PADDR);
const GICC_BASE: PhysAddr = pa!(axconfig::GICC_PADDR);
const GICH_BASE: PhysAddr = pa!(axconfig::GICH_PADDR);

static GICD: SpinNoIrq<GicDistributor> =
    SpinNoIrq::new(GicDistributor::new(phys_to_virt(GICD_BASE).as_mut_ptr()));

static GICH: SpinNoIrq<GicHypervisorInterface> = SpinNoIrq::new(GicHypervisorInterface::new(
    phys_to_virt(GICH_BASE).as_mut_ptr(),
));

// per-CPU, no lock
static GICC: GicCpuInterface = GicCpuInterface::new(phys_to_virt(GICC_BASE).as_mut_ptr());

/// Enables or disables the given IRQ.
pub fn set_enable(irq_num: usize, enabled: bool) {
    trace!("GICD set enable: {} {}", irq_num, enabled);
    GICD.lock().set_enable(irq_num as _, enabled);
}

/// Registers an IRQ handler for the given IRQ.
///
/// It also enables the IRQ if the registration succeeds. It returns `false` if
/// the registration failed.
pub fn register_handler(irq_num: usize, handler: IrqHandler) -> bool {
    trace!("register handler irq {}", irq_num);
    crate::irq::register_handler_common(irq_num, handler)
}

/// Fetches the IRQ number.
pub fn fetch_irq() -> usize {
    GICC.iar() as usize
}

/// Dispatches the IRQ.
///
/// This function is called by the common interrupt handler. It looks
/// up in the IRQ handler table and calls the corresponding handler. If
/// necessary, it also acknowledges the interrupt controller after handling.
pub fn dispatch_irq(irq_no: usize) {
    if irq_no == 0 {
        GICC.handle_irq(|irq_num| crate::irq::dispatch_irq_common(irq_num as _));
    } else {
        crate::irq::dispatch_irq_common(irq_no as _);
        GICC.eoi(irq_no as _);
        GICC.dir(irq_no as _);
    }
}

/// Initializes GICD, GICC on the primary CPU.
pub(crate) fn init_primary() {
    info!("Initialize GICv2...");
    GICD.lock().init();
    GICC.init();
}

/// Initializes GICC on secondary CPUs.
#[cfg(feature = "smp")]
pub(crate) fn init_secondary() {
    GICC.init();
}

pub struct GicInterface {}

/// Sets the value of a List Register (LR) at the specified index.
///
/// This function locks the Generic Interrupt Controller Hypervisor (GICH) interface
/// and sets the specified List Register to the provided value.
///
/// # Arguments
///
/// * `idx` - The index of the List Register to set.
/// * `lr` - The value to set in the List Register.
impl GicInterface {
    pub fn set_lr(&self, idx: usize, lr: u32) {
        GICH.lock().set_lr(idx, lr);
    }
}
