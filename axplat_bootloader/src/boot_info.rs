//! Boot information passed from bootloader to kernel

use crate::memory::{PhysAddr, VirtAddr};

/// Boot protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BootProtocol {
    /// Device Tree boot (X0 = DTB address)
    DeviceTree = 1,
    /// UEFI boot (X0 = ImageHandle, X1 = SystemTable)
    UEFI = 2,
    /// Multiboot boot (x86)
    Multiboot = 3,
    /// BIOS boot (x86)
    BIOS = 4,
}

/// Physical memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MemoryType {
    /// Usable memory
    Usable = 1,
    /// Reserved memory
    Reserved = 2,
    /// ACPI reclaimable memory
    AcpiReclaimable = 3,
    /// ACPI NVS memory
    AcpiNvs = 4,
    /// Bad memory
    BadMemory = 5,
    /// Memory used by bootloader
    BootloaderReserved = 6,
    /// Memory occupied by kernel image
    Kernel = 7,
}

/// Physical memory region
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryRegion {
    /// Start physical address
    pub start: PhysAddr,
    /// Size in bytes
    pub size: usize,
    /// Memory type
    pub memory_type:  MemoryType,
}

impl MemoryRegion {
    pub const fn new(start: PhysAddr, size: usize, memory_type: MemoryType) -> Self {
        Self {
            start,
            size,
            memory_type,
        }
    }

    /// End address (exclusive)
    pub const fn end(&self) -> PhysAddr {
        self.start + self.size
    }

    /// Check if the region contains the specified address
    pub const fn contains(&self, addr: PhysAddr) -> bool {
        addr >= self.start && addr < self.end()
    }
}

/// Boot information passed from bootloader to kernel
#[repr(C)]
pub struct BootInfo {
    /// Magic number:  "AXBT" (0x54425841)
    pub magic: u32,

    /// BootInfo structure version
    pub version: u32,

    /// Boot protocol
    pub boot_protocol: BootProtocol,

    /// Physical address where kernel is loaded
    pub kernel_phys_base: PhysAddr,

    /// Virtual address where kernel is currently running
    pub kernel_virt_base: VirtAddr,

    /// Linear mapping offset (VA = PA + offset)
    pub linear_map_offset: usize,

    /// Number of physical memory regions
    pub memory_region_count: usize,

    /// Pointer to physical memory region array
    pub memory_regions_ptr: *const MemoryRegion,

    /// Device Tree Blob physical address (if available)
    pub dtb_phys_addr: usize,

    /// UEFI System Table physical address (if available)
    pub uefi_system_table:  usize,

    /// Command line arguments pointer (C string)
    pub cmdline_ptr: *const u8,

    /// Command line arguments length
    pub cmdline_len: usize,
}

impl BootInfo {
    /// Magic number: "AXBT"
    pub const MAGIC: u32 = 0x54425841;

    /// Current version number
    pub const VERSION: u32 = 1;

    /// Create empty BootInfo
    pub const fn new(boot_protocol: BootProtocol) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            boot_protocol,
            kernel_phys_base: 0,
            kernel_virt_base: 0,
            linear_map_offset: 0,
            memory_region_count: 0,
            memory_regions_ptr: core::ptr:: null(),
            dtb_phys_addr: 0,
            uefi_system_table: 0,
            cmdline_ptr: core::ptr::null(),
            cmdline_len:  0,
        }
    }

    /// Get memory region list
    pub fn memory_regions(&self) -> &[MemoryRegion] {
        if self.memory_regions_ptr.is_null() {
            &[]
        } else {
            unsafe {
                core::slice::from_raw_parts(
                    self.memory_regions_ptr,
                    self.memory_region_count
                )
            }
        }
    }

    /// Get command line arguments
    pub fn cmdline(&self) -> Option<&str> {
        if self.cmdline_ptr.is_null() || self.cmdline_len == 0 {
            None
        } else {
            unsafe {
                let slice = core::slice::from_raw_parts(
                    self.cmdline_ptr,
                    self.cmdline_len
                );
                core::str::from_utf8(slice).ok()
            }
        }
    }

    /// Verify magic and version
    pub fn is_valid(&self) -> bool {
        self.magic == Self:: MAGIC && self.version == Self::VERSION
    }
}
