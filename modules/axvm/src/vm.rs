use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use hypercraft::{VCpu, VmCpus, VM};

use super::arch::new_vcpu;
#[cfg(target_arch = "x86_64")]
use super::device::{self, NimbosVmDevices, X64VcpuDevices, X64VmDevices};
use crate::GuestPageTable;
use alloc::sync::Arc;
use axhal::{current_cpu_id, hv::HyperCraftHalImpl};

use crate::config::entry::vm_cfg_entry;
use crate::device::BarAllocImpl;

// use super::type1_5::cell;
static INIT_GPM_OK: AtomicU32 = AtomicU32::new(0);
static INITED_CPUS: AtomicUsize = AtomicUsize::new(0);

pub fn config_boot_linux() {
    let hart_id = current_cpu_id();
    let linux_context = axhal::hv::get_linux_context();

    crate::arch::cpu_hv_hardware_enable(hart_id, linux_context)
        .expect("cpu_hv_hardware_enable failed");

    if hart_id == 0 {
        super::config::init_root_gpm().expect("init_root_gpm failed");
        INIT_GPM_OK.store(1, Ordering::Release);
    } else {
        while INIT_GPM_OK.load(Ordering::Acquire) < 1 {
            core::hint::spin_loop();
        }
    }

    let ept = super::config::root_gpm().nest_page_table();
    let ept_root = super::config::root_gpm().nest_page_table_root();

    let vcpu = new_vcpu(
        hart_id,
        crate::arch::cpu_vmcs_revision_id(),
        ept_root,
        &linux_context,
    )
    .unwrap();
    let mut vcpus =
        VmCpus::<HyperCraftHalImpl, X64VcpuDevices<HyperCraftHalImpl, BarAllocImpl>>::new();
    info!("CPU{} add vcpu to vm...", hart_id);
    vcpus.add_vcpu(vcpu).expect("add vcpu failed");
    let mut vm = VM::<
        HyperCraftHalImpl,
        X64VcpuDevices<HyperCraftHalImpl, BarAllocImpl>,
        X64VmDevices<HyperCraftHalImpl, BarAllocImpl>,
        GuestPageTable,
    >::new(vcpus, Arc::new(ept));
    // The bind_vcpu method should be decoupled with vm struct.
    vm.bind_vcpu(hart_id).expect("bind vcpu failed");

    INITED_CPUS.fetch_add(1, Ordering::SeqCst);
    while INITED_CPUS.load(Ordering::Acquire) < axconfig::SMP {
        core::hint::spin_loop();
    }

    debug!("CPU{} before run vcpu", hart_id);
    info!("{:?}", vm.run_type15_vcpu(hart_id, &linux_context));

    // disable hardware virtualization todo
}

pub fn boot_vm(vm_id: usize) {
    let vm_cfg_entry = match vm_cfg_entry(vm_id) {
        Some(entry) => entry,
        None => {
            warn!("VM {} not existed, boot vm failed", vm_id);
            return;
        }
    };

    info!(
        "boot_vm {} {:?} on core {}, guest entry {:#x}",
        vm_id,
        vm_cfg_entry.get_vm_type(),
        axhal::current_cpu_id(),
        vm_cfg_entry.get_vm_entry(),
    );

    let gpm = vm_cfg_entry
        .generate_guest_phys_memory_set()
        .expect("Failed to generate GPM");

    let npt = gpm.nest_page_table();
    let npt_root = gpm.nest_page_table_root();
    info!("{:#x?}", gpm);
    // Main scheduling item, managed by `axtask`
    let vcpu = VCpu::new(
        0,
        crate::arch::cpu_vmcs_revision_id(),
        vm_cfg_entry.get_vm_entry(),
        npt_root,
    )
    .unwrap();
    let mut vcpus =
        VmCpus::<HyperCraftHalImpl, X64VcpuDevices<HyperCraftHalImpl, BarAllocImpl>>::new();
    vcpus.add_vcpu(vcpu).expect("add vcpu failed");
    let mut vm = VM::<
        HyperCraftHalImpl,
        X64VcpuDevices<HyperCraftHalImpl, BarAllocImpl>,
        NimbosVmDevices<HyperCraftHalImpl, BarAllocImpl>,
        GuestPageTable,
    >::new(vcpus, Arc::new(npt));
    // The bind_vcpu method should be decoupled with vm struct.
    vm.bind_vcpu(0).expect("bind vcpu failed");

    info!("Running guest...");
    info!("{:?}", vm.run_vcpu(0));
}
