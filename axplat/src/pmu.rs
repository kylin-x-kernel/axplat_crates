/// Trait for PmuIf

use axcpu::TrapFrame;

/// PMU counter overflow callback.
///
/// Called in interrupt context.
pub type OverflowHandler = fn(&TrapFrame);

#[def_plat_interface]
pub trait PmuIf{
    /// Pmu interrupt handle func
    fn handle_overflows(tf: &TrapFrame) -> bool;

    /// Register an overflow handler for a PMU counter.
    fn register_overflow_handler(index: u32, handler: OverflowHandler) -> bool;
}