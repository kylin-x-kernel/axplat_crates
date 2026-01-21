//! UEFI boot handling

use crate::BootInfo;

/// Handle UEFI boot
pub fn handle_uefi_boot(boot_info: &mut BootInfo, image_handle: usize, system_table: usize) {
    info!("Handling UEFI boot, SystemTable at {:#x}", system_table);

    // TODO:
    // 1. Get memory map through UEFI protocol
    // 2. Fill boot_info.memory_regions
    // 3. Set up temporary page tables (linear mapping)
    // 4. Exit UEFI Boot Services
    // 5. Jump to kernel

    unimplemented!("UEFI boot handling not yet implemented");
}
