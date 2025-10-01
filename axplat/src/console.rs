//! Console input and output.

use core::{
    fmt::{Arguments, Result, Write},
    sync::atomic::{AtomicUsize, Ordering},
};

/// Console input and output interface.
#[def_plat_interface]
pub trait ConsoleIf {
    /// Writes given bytes to the console.
    fn write_bytes(bytes: &[u8]);

    /// Reads bytes from the console into the given mutable slice.
    ///
    /// Returns the number of bytes read.
    fn read_bytes(bytes: &mut [u8]) -> usize;
}

struct EarlyConsole;

impl Write for EarlyConsole {
    fn write_str(&mut self, s: &str) -> Result {
        write_bytes(s.as_bytes());
        Ok(())
    }
}

/// Lock for console operations to prevent mixed output from concurrent execution
pub static CONSOLE_LOCK: kspin::SpinNoIrq<()> = kspin::SpinNoIrq::new(());

/// Simple console print operation.
#[macro_export]
macro_rules! console_print {
    ($($arg:tt)*) => {
        $crate::console::__simple_print(format_args!($($arg)*));
    }
}

/// Simple console print operation, with a newline.
#[macro_export]
macro_rules! console_println {
    () => { $crate::ax_print!("\n") };
    ($($arg:tt)*) => {
        $crate::console::__simple_print(format_args!("{}\n", format_args!($($arg)*)));
    }
}

#[doc(hidden)]
pub fn __simple_print(fmt: Arguments) {
    let _guard = CONSOLE_LOCK.lock();
    EarlyConsole.write_fmt(fmt).unwrap();
    drop(_guard);
}

#[cfg(feature = "irq")]
static CONSOLE_IRQ: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "irq")]
pub fn init_console_irq(irq: usize) {
    CONSOLE_IRQ.store(irq, Ordering::SeqCst);
}

#[cfg(feature = "irq")]
pub fn get_console_irq() -> Option<usize> {
    let irq = CONSOLE_IRQ.load(Ordering::SeqCst);
    if irq != 0 { Some(irq) } else { None }
}
