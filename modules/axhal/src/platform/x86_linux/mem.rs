// TODO: get memory regions from multiboot info.

use crate::mem::{virt_to_phys, MemRegion, MemRegionFlags, PhysAddr, VirtAddr};

use super::config::HvSystemConfig;
use super::consts::{HV_BASE, PER_CPU_ARRAY_PTR};
use super::header::HvHeader;

// MemRegion {
//     paddr: virt_to_phys((_stext as usize).into()),
//     size: _etext as usize - _stext as usize,
//     flags: MemRegionFlags::RESERVED | MemRegionFlags::READ | MemRegionFlags::EXECUTE,
//     name: ".text",
// },

extern "C" {
    fn __header_start();
    fn __header_end();
}

/// Returns the vmm free memory regions (kernel image end to physical memory end).
fn vmm_free_regions() -> impl Iterator<Item = MemRegion> {
    let mem_pool_start = super::consts::free_memory_start();
    let mem_pool_end = super::consts::hv_end().align_down_4k();
    let mem_pool_size = mem_pool_end.as_usize() - mem_pool_start.as_usize();
    core::iter::once(MemRegion {
        paddr: virt_to_phys(mem_pool_start),
        size: mem_pool_size,
        flags: MemRegionFlags::FREE | MemRegionFlags::READ | MemRegionFlags::WRITE,
        name: "free memory",
    })
}

fn vmm_per_cpu_data_regions() -> impl Iterator<Item = MemRegion> {
    let start_paddr = virt_to_phys(VirtAddr::from(PER_CPU_ARRAY_PTR as usize));
    let end_paddr = virt_to_phys(super::consts::free_memory_start());

    let size = end_paddr.as_usize() - start_paddr.as_usize();

    core::iter::once(MemRegion {
        paddr: start_paddr,
        size,
        flags: MemRegionFlags::READ | MemRegionFlags::WRITE,
        name: "per-CPU data, configurations",
    })
}

/// Returns platform-specific memory regions.
pub(crate) fn platform_regions() -> impl Iterator<Item = MemRegion> {
    // Add region for HvHeader.
    // See modules/axhal/linker_hv.lds.S for details.
    core::iter::once(MemRegion {
        paddr: virt_to_phys((__header_start as usize).into()),
        size: __header_end as usize - __header_start as usize,
        flags: MemRegionFlags::RESERVED | MemRegionFlags::READ,
        name: ".header",
    })
    .chain(vmm_per_cpu_data_regions())
    .chain(vmm_free_regions())
    // Here we do not use the `default_free_regions`` from  `crate::mem`.
    // Cause we need to reserved regions for
    // per-CPU data and `HvSystemConfig`
    .chain(crate::mem::default_mmio_regions())
}
