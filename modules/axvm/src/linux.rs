/// Temporar module to boot Linux as a guest VM.
///
/// To be removed...
// use hypercraft::GuestPageTableTrait;
use hypercraft::{PerCpu, VCpu, VmCpus, VM};

use crate::mm::GuestPhysMemorySet;

#[cfg(target_arch = "x86_64")]
use super::device::{self, X64VcpuDevices, X64VmDevices};

use axhal::hv::HyperCraftHalImpl;

static mut LINUX_GUEST_GPM: Option<GuestPhysMemorySet> = None;
pub(crate) fn get_linux_gpm() -> &'static mut GuestPhysMemorySet {
    unsafe { LINUX_GUEST_GPM.as_mut().unwrap() }
}

pub fn config_boot_linux(hart_id: usize) {
    info!("into main {}", hart_id);

    // Fix: this function shoule be moved to somewhere like vm_entry.
    crate::arch::cpu_hv_hardware_enable(hart_id);

    // Alloc guest memory set.
    // Fix: this should be stored inside VM structure.
    unsafe {
        LINUX_GUEST_GPM.replace(super::config::setup_gpm(hart_id).unwrap());
    }
    let npt = unsafe { LINUX_GUEST_GPM.as_ref().map(|g| g.nest_page_table_root()).unwrap() };
    info!("{:#x?}", unsafe { npt });

    // Main scheduling item, managed by `axtask`
    let vcpu = VCpu::new(0, crate::arch::cpu_vmcs_revision_id(), 0x7c00, npt).unwrap();

    let mut vcpus = VmCpus::<HyperCraftHalImpl, X64VcpuDevices<HyperCraftHalImpl>>::new();
    vcpus.add_vcpu(vcpu).expect("add vcpu failed");

    let mut vm = VM::<
        HyperCraftHalImpl,
        X64VcpuDevices<HyperCraftHalImpl>,
        X64VmDevices<HyperCraftHalImpl>,
    >::new(vcpus);
	
	// The bind_vcpu method should be decoupled with vm struct.
    vm.bind_vcpu(0).expect("bind vcpu failed");

    if hart_id == 0 {
        let (_, dev) = vm.get_vcpu_and_device(0).unwrap();
        // *(dev.console.lock().backend()) = device::device_emu::MultiplexConsoleBackend::Primary;

        for v in 0..256 {
            crate::irq::set_host_irq_enabled(v, true);
        }
    }

    info!("Running guest...");
    info!("{:?}", vm.run_vcpu(0));

    crate::arch::cpu_hv_hardware_disable();

    panic!("done");
}
