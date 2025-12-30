use aarch64_pmuv3::pmuv3::{PmuCounter, PmuEvent};
use lazyinit::LazyInit;

const MAX_PMU_COUNTERS: usize = 32;

/// Per-CPU PMU manager.
///
/// This structure tracks the lifecycle of PMU counters on a per-CPU basis.
/// Each slot may or may not be initialized, hence the use of `Option`.
pub struct PmuManager {
    counters: [Option<PmuCounter>; MAX_PMU_COUNTERS],
}

/// Per-CPU lazy-initialized PMU manager.
///
/// The PMU is brought up on demand for each CPU.
#[percpu::def_percpu]
static PMU: LazyInit<PmuManager> = LazyInit::new();

/// Ensure that the per-CPU PMU manager is initialized.
///
/// This performs one-time initialization per CPU and returns
/// a mutable reference to the PMU manager.
#[inline]
unsafe fn ensure_pmu_inited() -> &'static mut PmuManager {
    let pmu = unsafe { PMU.current_ref_mut_raw() };
    pmu.call_once(|| PmuManager {
        counters: [const { None }; MAX_PMU_COUNTERS],
    });
    pmu
}

/// Initialize the cycle counter.
///
/// The cycle counter is mapped to the last PMU counter slot.
/// Returns `false` if the counter is already initialized or
/// if the underlying PMU is not supported.
pub fn init_cycle_counter(threshold: u64) -> bool {
    unsafe {
        let pmu_mgr = ensure_pmu_inited();

        let idx = MAX_PMU_COUNTERS - 1;

        // Do not overwrite an existing counter.
        if pmu_mgr.counters[idx].is_some() {
            return false;
        }

        let counter = PmuCounter::new_cycle_counter(threshold);

        // Bail out early if PMU is not supported on this CPU.
        if counter.check_pmu_support().is_err() {
            return false;
        }

        pmu_mgr.counters[idx] = Some(counter);
        true
    }
}

/// Initialize an event counter at the given index.
///
/// Event counters must not use the cycle counter slot.
/// Returns `false` on invalid index, duplicate initialization,
/// or unsupported PMU hardware.
pub fn init_event_counter(index: u32, threshold: u64, event: PmuEvent) -> bool {
    let idx = index as usize;

    // Reserve the last slot for the cycle counter.
    if idx >= MAX_PMU_COUNTERS - 1 {
        return false;
    }

    unsafe {
        let pmu_mgr = ensure_pmu_inited();

        // Do not overwrite an existing counter.
        if pmu_mgr.counters[idx].is_some() {
            return false;
        }

        let counter = PmuCounter::new_event_counter(index, threshold, event);

        // Check PMU availability before installing the counter.
        if counter.check_pmu_support().is_err() {
            return false;
        }

        pmu_mgr.counters[idx] = Some(counter);
        true
    }
}

/// Apply a mutable operation to a PMU counter if it exists.
///
/// Invalid indices or uninitialized counters are silently ignored.
/// This helper centralizes unsafe access and bounds checking.
#[inline]
unsafe fn with_counter_mut<F>(index: u32, f: F)
where
    F: FnOnce(&mut PmuCounter),
{
    if let Some(Some(counter)) =
        unsafe { PMU.current_ref_mut_raw().counters.get_mut(index as usize) }
    {
        f(counter);
    }
}

/// Enable the specified PMU counter.
///
/// This is a best-effort operation and is a no-op if the counter
/// does not exist or is not initialized.
pub fn enable(index: u32) {
    unsafe {
        with_counter_mut(index, |c| c.enable());
    }
}

/// Disable the specified PMU counter.
///
/// This is a best-effort operation and is a no-op if the counter
/// does not exist or is not initialized.
pub fn disable(index: u32) {
    unsafe {
        with_counter_mut(index, |c| c.disable());
    }
}

/// Query whether the specified PMU counter is enabled.
///
/// Returns `false` if the index is invalid or the counter
/// has not been initialized.
pub fn is_enabled(index: u32) -> bool {
    unsafe {
        PMU.current_ref_mut_raw()
            .counters
            .get(index as usize)
            .and_then(|c| c.as_ref())
            .map(|c| c.is_enabled())
            .unwrap_or(false)
    }
}

/// Handle a PMU counter overflow.
///
/// Returns `true` if an initialized counter handled the overflow
/// successfully, otherwise returns `false`.
pub fn handle_overflow(index: u32) -> bool {
    unsafe {
        let mut handled = false;
        with_counter_mut(index, |c| {
            handled = c.handle_overflow().is_ok();
        });
        handled
    }
}

/// Update the overflow threshold of a PMU counter.
///
/// This operation is ignored if the counter does not exist
/// or has not been initialized.
pub fn set_threshold(index: u32, threshold: u64) {
    unsafe {
        with_counter_mut(index, |c| c.set_threshold(threshold));
    }
}