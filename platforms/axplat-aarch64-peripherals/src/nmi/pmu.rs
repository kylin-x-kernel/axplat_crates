// =============================================================================
// PMU-based NMI Source
// =============================================================================
use aarch64_pmuv3::pmuv3::PmuCounter;
use lazyinit::LazyInit;

#[percpu::def_percpu]
static PMU: LazyInit<PmuCounter> = LazyInit::new();

pub fn init(threshold: u64) -> bool {
    let pmu = PmuCounter::new_cycle_counter(threshold);
    unsafe {
        PMU.current_ref_mut_raw().call_once(|| pmu);
        PMU.current_ref_mut_raw().check_pmu_support().is_ok()
    }
}

pub fn enable() {
    unsafe {
        PMU.current_ref_mut_raw().enable();
    }
}

pub fn disable() {
    unsafe {
        PMU.current_ref_mut_raw().disable();
    }
}

pub fn is_enabled() -> bool {
    unsafe { PMU.current_ref_mut_raw().is_enabled() }
}

pub fn handle_overflow() -> bool {
    unsafe { PMU.current_ref_mut_raw().handle_overflow().is_ok() }
}

/// Default implementation of [`axplat::nmi::NmiIf`] using the GIC.
#[macro_export]
macro_rules! nmi_if_impl {
    ($name:ident) => {
        struct $name;

        use axplat::nmi::NmiType;

        #[impl_plat_interface]
        impl axplat::nmi::NmiIf for $name {
            fn init(threshold: u64) -> bool {
                $crate::nmi::pmu::init(threshold)
            }

            fn enable() {
                $crate::nmi::pmu::enable();
            }

            fn disable() {
                $crate::nmi::pmu::disable();
            }

            fn is_enabled() -> bool {
                $crate::nmi::pmu::is_enabled()
            }

            fn handle() -> bool {
                $crate::nmi::pmu::handle_overflow()
            }

            fn name() -> &'static str {
                "PMU"
            }

            fn nmi_type() -> NmiType {
                NmiType::PseudoNmi
            }
        }
    };
}
