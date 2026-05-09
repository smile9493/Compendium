use std::panic::catch_unwind;
use std::sync::{LazyLock, Mutex};

use tracing::{error, warn};

use crate::error::{PdfiumGuardError, PdfiumGuardResult};

/// Global serialization lock for Pdfium FFI calls.
///
/// Pdfium's C++ library is NOT thread-safe. All FFI calls must be serialized.
/// This global mutex provides serialization for the standalone `catch_pdfium()`
/// function when a `PdfiumGuard` instance is not available.
static PDFIUM_GLOBAL_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Pdfium FFI safety guard.
///
/// All calls that enter Pdfium must go through `safe_execute`.
/// The internal `Mutex<()>` serialises concurrent access so only one thread
/// ever enters the FFI boundary, and `catch_unwind` prevents panics from
/// crossing the FFI boundary.
pub struct PdfiumGuard {
    lock: Mutex<()>,
}

impl PdfiumGuard {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(()),
        }
    }

    /// Execute a closure inside the Pdfium serialisation + panic-isolation boundary.
    pub fn safe_execute<F, R>(&self, f: F) -> PdfiumGuardResult<R>
    where
        F: FnOnce() -> R + std::panic::UnwindSafe,
    {
        let guard = self.lock.lock().map_err(|_| {
            warn!("PdfiumGuard mutex poisoned - lock is contaminated");
            PdfiumGuardError::LockPoisoned
        })?;

        let result = catch_unwind(f);

        drop(guard);

        result.map_err(|_| {
            error!("Pdfium FFI call panicked - caught by PdfiumGuard");
            PdfiumGuardError::Panic
        })
    }
}

impl Default for PdfiumGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience: wrap a Pdfium render call with catch_unwind and global mutex serialization.
///
/// This is the primary public helper that callers should use for one-shot
/// FFI calls without needing to hold a `PdfiumGuard` reference.
///
/// Uses a global `Mutex<()>` to serialize all Pdfium FFI access, since Pdfium's
/// C++ library is NOT thread-safe. Without this serialization, concurrent FFI
/// calls would cause data corruption or UB (P0 violation per ref/15-ffi-interop).
pub fn catch_pdfium<F, R>(f: F) -> PdfiumGuardResult<R>
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    // SAFETY: We must hold the global lock before entering Pdfium FFI.
    // If the lock is poisoned (a previous panic in Pdfium FFI), we return
    // an error instead of proceeding, which would be UB on corrupted state.
    let guard = PDFIUM_GLOBAL_LOCK.lock().map_err(|_| {
        error!("Pdfium global mutex poisoned - lock is contaminated");
        PdfiumGuardError::LockPoisoned
    })?;

    let result = catch_unwind(f);

    drop(guard);

    result.map_err(|_| {
        error!("Pdfium FFI call panicked");
        PdfiumGuardError::Panic
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_execute_ok() {
        let guard = PdfiumGuard::new();
        let result = guard.safe_execute(|| 42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn safe_execute_catches_panic() {
        let guard = PdfiumGuard::new();
        let result: PdfiumGuardResult<()> = guard.safe_execute(|| {
            panic!("ffi boom");
        });
        assert!(matches!(result, Err(PdfiumGuardError::Panic)));
    }

    #[test]
    fn catch_pdfium_ok() {
        let r = catch_pdfium(|| "hello");
        assert_eq!(r.unwrap(), "hello");
    }

    #[test]
    fn catch_pdfium_panic() {
        let r: PdfiumGuardResult<()> = catch_pdfium(|| panic!("boom"));
        assert!(matches!(r, Err(PdfiumGuardError::Panic)));
    }
}
