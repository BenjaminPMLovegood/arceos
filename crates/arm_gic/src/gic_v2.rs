//! Types and definitions for GICv2.
//!
//! The official documentation: <https://developer.arm.com/documentation/ihi0048/latest/>

use core::ptr::NonNull;

use crate::{TriggerMode, GIC_MAX_IRQ, SPI_RANGE};
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};

register_structs! {
    /// GIC Distributor registers.
    #[allow(non_snake_case)]
    pub GicDistributor {
        /// Distributor Control Register.
        (0x0000 => CTLR: ReadWrite<u32>),
        /// Interrupt Controller Type Register.
        (0x0004 => TYPER: ReadOnly<u32>),
        /// Distributor Implementer Identification Register.
        (0x0008 => IIDR: ReadOnly<u32>),
        (0x000c => _reserved_0),
        /// Interrupt Group Registers.
        (0x0080 => IGROUPR: [ReadWrite<u32>; 0x20]),
        /// Interrupt Set-Enable Registers.
        (0x0100 => ISENABLER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Clear-Enable Registers.
        (0x0180 => ICENABLER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Set-Pending Registers.
        (0x0200 => ISPENDR: [ReadWrite<u32>; 0x20]),
        /// Interrupt Clear-Pending Registers.
        (0x0280 => ICPENDR: [ReadWrite<u32>; 0x20]),
        /// Interrupt Set-Active Registers.
        (0x0300 => ISACTIVER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Clear-Active Registers.
        (0x0380 => ICACTIVER: [ReadWrite<u32>; 0x20]),
        /// Interrupt Priority Registers.
        (0x0400 => IPRIORITYR: [ReadWrite<u32>; 0x100]),
        /// Interrupt Processor Targets Registers.
        (0x0800 => ITARGETSR: [ReadWrite<u32>; 0x100]),
        /// Interrupt Configuration Registers.
        (0x0c00 => ICFGR: [ReadWrite<u32>; 0x40]),
        (0x0d00 => _reserved_1),
        /// Software Generated Interrupt Register.
        (0x0f00 => SGIR: WriteOnly<u32>),
        (0x0f04 => @END),
    }
}

register_structs! {
    /// GIC CPU Interface registers.
    #[allow(non_snake_case)]
    pub GicCpuInterface {
        /// CPU Interface Control Register.
        (0x0000 => CTLR: ReadWrite<u32>),
        /// Interrupt Priority Mask Register.
        (0x0004 => PMR: ReadWrite<u32>),
        /// Binary Point Register.
        (0x0008 => BPR: ReadWrite<u32>),
        /// Interrupt Acknowledge Register.
        (0x000c => IAR: ReadOnly<u32>),
        /// End of Interrupt Register.
        (0x0010 => EOIR: WriteOnly<u32>),
        /// Running Priority Register.
        (0x0014 => RPR: ReadOnly<u32>),
        /// Highest Priority Pending Interrupt Register.
        (0x0018 => HPPIR: ReadOnly<u32>),
        (0x001c => _reserved_1),
        /// CPU Interface Identification Register.
        (0x00fc => IIDR: ReadOnly<u32>),
        (0x0100 => _reserved_2),
        /// Deactivate Interrupt Register.
        (0x1000 => DIR: WriteOnly<u32>),
        (0x1004 => @END),
    }
}

/// The GIC distributor.
///
/// The Distributor block performs interrupt prioritization and distribution
/// to the CPU interface blocks that connect to the processors in the system.
///
/// The Distributor provides a programming interface for:
/// - Globally enabling the forwarding of interrupts to the CPU interfaces.
/// - Enabling or disabling each interrupt.
/// - Setting the priority level of each interrupt.
/// - Setting the target processor list of each interrupt.
/// - Setting each peripheral interrupt to be level-sensitive or edge-triggered.
/// - Setting each interrupt as either Group 0 or Group 1.
/// - Forwarding an SGI to one or more target processors.
///
/// In addition, the Distributor provides:
/// - visibility of the state of each interrupt
/// - a mechanism for software to set or clear the pending state of a peripheral
///   interrupt.

/* 
#[derive(Debug, Clone)]
pub struct GicDistributor {
    base: NonNull<GicDistributorRegs>,
    max_irqs: usize,
}
*/
// type GicDistributor = GicDistributorRegs;

/// The GIC CPU interface.
///
/// Each CPU interface block performs priority masking and preemption
/// handling for a connected processor in the system.
///
/// Each CPU interface provides a programming interface for:
///
/// - enabling the signaling of interrupt requests to the processor
/// - acknowledging an interrupt
/// - indicating completion of the processing of an interrupt
/// - setting an interrupt priority mask for the processor
/// - defining the preemption policy for the processor
/// - determining the highest priority pending interrupt for the processor.

/* 
#[derive(Debug, Clone)]
pub struct GicCpuInterface {
    base: NonNull<GicCpuInterfaceRegs>,
}
*/

// type GicCpuInterface = GicCpuInterfaceRegs;


unsafe impl Send for GicDistributor {}
unsafe impl Sync for GicDistributor {}

unsafe impl Send for GicCpuInterface {}
unsafe impl Sync for GicCpuInterface {}

use crate::device_ref::*;

pub static mut GICD: DeviceRef<GicDistributor> = unsafe { DeviceRef::new() };
pub static mut GICC: DeviceRef<GicCpuInterface> = unsafe { DeviceRef::new() };
static GICD_LOCK: spinlock::SpinNoIrq<()> = spinlock::SpinNoIrq::new(());

impl GicDistributor {
    /// Construct a new GIC distributor instance from the base address.

    pub fn init_base(base: *mut u8) {
        unsafe { GICD.dev_init(base as * const GicDistributor); }
    }
    
    /// The number of implemented CPU interfaces.
    pub fn cpu_num(&self) -> usize {
        ((self.TYPER.get() as usize >> 5) & 0b111) + 1
    }

    /// The maximum number of interrupts that the GIC supports
    pub fn max_irqs(&self) -> usize {
        ((self.TYPER.get() as usize & 0b11111) + 1) * 32
    }

    /// Configures the trigger mode for the given interrupt.
    pub fn configure_interrupt(&mut self, vector: usize, tm: TriggerMode) {
        // Only configurable for SPI interrupts
        if vector < SPI_RANGE.start {
            return;
        }

        // type is encoded with two bits, MSB of the two determine type
        // 16 irqs encoded per ICFGR register
        let reg_idx = vector >> 4;
        let bit_shift = ((vector & 0xf) << 1) + 1;
        
        let lock = GICD_LOCK.lock();
        let mut reg_val = self.ICFGR[reg_idx].get();
        match tm {
            TriggerMode::Edge => reg_val |= 1 << bit_shift,
            TriggerMode::Level => reg_val &= !(1 << bit_shift),
        }
        self.ICFGR[reg_idx].set(reg_val);
        drop(lock);
    }

    /// Enables or disables the given interrupt.
    pub fn set_enable(&mut self, vector: usize, enable: bool) {
        // if vector >= self.max_irqs {
        //     return;
        // }

        let reg = vector / 32;
        let mask = 1 << (vector % 32);

        let lock = GICD_LOCK.lock();
        if enable {
            self.ISENABLER[reg].set(mask);
        } else {
            self.ICENABLER[reg].set(mask);
        }
        drop(lock);
    }

    /// Get interrupt priority.
    pub fn get_priority(&self, int_id: usize) -> usize {
        let idx = (int_id * 8) / 32;
        let off = (int_id * 8) % 32;
        ((self.IPRIORITYR[idx].get() >> off) & 0xff) as usize
    }

    /// Set interrupt priority.
    pub fn set_priority(&self, int_id: usize, priority: u8) {
        let idx = (int_id * 8) / 32;
        let offset = (int_id * 8) % 32;
        let mask: u32 = 0xff << offset;

        let lock = GICD_LOCK.lock();
        let prev_reg_val = self.IPRIORITYR[idx].get();
        // clear target int_id priority and set its priority.
        let reg_val = (prev_reg_val & !mask) | (((priority as u32) << offset) & mask);
        self.IPRIORITYR[idx].set(reg_val);
        drop(lock);
    }

    /// Initializes the GIC distributor.
    ///
    /// It disables all interrupts, sets the target of all SPIs to CPU 0,
    /// configures all SPIs to be edge-triggered, and finally enables the GICD.
    ///
    /// This function should be called only once.
    pub fn init(&mut self) {
        let max_irqs = self.max_irqs();
        assert!(max_irqs <= GIC_MAX_IRQ);;

        // Disable all interrupts
        for i in (0..max_irqs).step_by(32) {
            self.ICENABLER[i / 32].set(u32::MAX);
            self.ICPENDR[i / 32].set(u32::MAX);
            self.ICACTIVER[i / 32].set(u32::MAX);
        }
        if self.cpu_num() > 1 {
            for i in (SPI_RANGE.start..max_irqs).step_by(4) {
                // Set external interrupts to target cpu 0
                self.ITARGETSR[i / 4].set(0x01_01_01_01);
                self.IPRIORITYR[i / 4].set(u32::MAX);
            }
        }
        // Initialize all the SPIs to edge triggered
        for i in SPI_RANGE.start..max_irqs {
            self.configure_interrupt(i, TriggerMode::Edge);
        }

        // enable GIC0
        self.CTLR.set(1);
    }
}

impl GicCpuInterface {

    pub fn init_base(base: *mut u8) {
        unsafe { GICC.dev_init(base as * const GicCpuInterface); }
    }


    /// Returns the interrupt ID of the highest priority pending interrupt for
    /// the CPU interface. (read GICC_IAR)
    ///
    /// The read returns a spurious interrupt ID of `1023` if the distributor
    /// or the CPU interface are disabled, or there is no pending interrupt on
    /// the CPU interface.
    pub fn iar(&self) -> u32 {
        self.IAR.get()
    }

    /// Informs the CPU interface that it has completed the processing of the
    /// specified interrupt. (write GICC_EOIR)
    ///
    /// The value written must be the value returns from [`Self::iar`].
    pub fn set_eoi(&self, iar: u32) {
        self.EOIR.set(iar);
    }

    /// handles the signaled interrupt.
    ///
    /// It first reads GICC_IAR to obtain the pending interrupt ID and then
    /// calls the given handler. After the handler returns, it writes GICC_EOIR
    /// to acknowledge the interrupt.
    ///
    /// If read GICC_IAR returns a spurious interrupt ID of `1023`, it does
    /// nothing.
    pub fn handle_irq<F>(&self, handler: F)
    where
        F: FnOnce(u32),
    {
        let iar = self.iar();
        let vector = iar & 0x3ff;
        if vector < 1020 {
            handler(vector);
            self.set_eoi(iar);
        } else {
            // spurious
        }
    }

    /// Initializes the GIC CPU interface.
    ///
    /// It unmask interrupts at all priority levels and enables the GICC.
    ///
    /// This function should be called only once.
    pub fn init(&self) {
        // enable GIC0
        self.CTLR.set(1);
        // #[cfg(feature = "hv")]
        // // set EOImodeNS and EN bit for hypervisor
        // self.CTLR.set(1| 0x200);
        // unmask interrupts at all priority levels
        self.PMR.set(0xff);
    }
}
