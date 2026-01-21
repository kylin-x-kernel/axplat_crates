//! AArch64 boot entry point

use core::arch::asm;
use crate::{BootInfo, boot_info::BootProtocol};
use crate::protocol::detect_boot_protocol;

/// Boot stack size
const BOOT_STACK_SIZE: usize = 64 * 1024;

/// Boot stack
#[unsafe(link_section = ".bss.stack")]
static mut BOOT_STACK: [u8; BOOT_STACK_SIZE] = [0; BOOT_STACK_SIZE];

/// Saved X0 register value
static mut SAVED_X0: usize = 0;

/// Saved X1 register value
static mut SAVED_X1: usize = 0;

/// AArch64 boot entry point
///
/// Register convention:
/// - X0 = DTB address OR UEFI ImageHandle
/// - X1 = 0 OR UEFI SystemTable
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub unsafe extern "C" fn _start() -> ! {
    asm!(
        // Save X0 and X1
        "adrp    x2, {saved_x0}",
        "str     x0, [x2, :lo12:{saved_x0}]",
        "adrp    x2, {saved_x1}",
        "str     x1, [x2, :lo12:{saved_x1}]",

        // Set up boot stack
        "adrp    x2, {boot_stack}",
        "add     x2, x2, :lo12:{boot_stack}",
        "mov     x3, {boot_stack_size}",
        "add     sp, x2, x3",

        // Jump to Rust code
        "b       {rust_main}",

        saved_x0 = sym SAVED_X0,
        saved_x1 = sym SAVED_X1,
        boot_stack = sym BOOT_STACK,
        boot_stack_size = const BOOT_STACK_SIZE,
        rust_main = sym rust_main,
        options(noreturn)
    )
}

/// Rust main entry point
#[unsafe(no_mangle)]
unsafe extern "C" fn rust_main() -> ! {
    // Get saved register values
    let x0 = unsafe { SAVED_X0 };
    let x1 = unsafe { SAVED_X1 };

    // Detect boot protocol
    let protocol = unsafe { detect_boot_protocol(x0, x1) };

    // Initialize BootInfo
    let mut boot_info = BootInfo::new(protocol);

    match protocol {
        BootProtocol::DeviceTree => {
            boot_info.dtb_phys_addr = x0;
            // TODO: Parse DTB, set up page tables, jump to kernel
            super::dtb::handle_devicetree_boot(&mut boot_info, x0);
        }
        BootProtocol::UEFI => {
            boot_info.uefi_system_table = x1;
            // TODO:  UEFI boot handling
            super::uefi::handle_uefi_boot(&mut boot_info, x0, x1);
        }
        _ => {
            panic!("Unsupported boot protocol:  {: ?}", protocol);
        }
    }

    // Jump to kernel (needs implementation)
    panic!("Bootloader:  kernel entry not implemented yet!");
}
