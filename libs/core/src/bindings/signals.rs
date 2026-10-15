//! Ctrl-C during engine work that runs without the GIL.

use std::sync::Mutex;

use pyo3::exceptions::PyKeyboardInterrupt;
use pyo3::prelude::*;

use crate::error::{CfError, Result};
use crate::search::Interrupt;

/// Pending Python signals as a search [`Interrupt`]. A poll briefly takes
/// the GIL and runs the signal handlers (`PyErr_CheckSignals`); the exception
/// a handler raises (`KeyboardInterrupt` for Ctrl-C) is kept to be raised
/// once the engine has stopped.
#[derive(Default)]
struct PythonSignals {
    raised: Mutex<Option<PyErr>>,
}

impl Interrupt for PythonSignals {
    fn interrupted(&self) -> bool {
        Python::with_gil(|py| match py.check_signals() {
            Ok(()) => false,
            Err(error) => {
                *self.raised.lock().expect("signal lock poisoned") = Some(error);
                true
            }
        })
    }
}

/// Run `work` without the GIL, handing it an [`Interrupt`] for pending
/// signals. When a signal handler raises, the search `work` runs stops and
/// the handler's exception is raised here.
pub(crate) fn interruptible<T, F>(py: Python<'_>, work: F) -> PyResult<T>
where
    T: Send,
    F: FnOnce(&dyn Interrupt) -> Result<T> + Send,
{
    let signals = PythonSignals::default();
    match py.allow_threads(|| work(&signals)) {
        Ok(value) => Ok(value),
        Err(CfError::Interrupted) => Err(signals
            .raised
            .into_inner()
            .expect("signal lock poisoned")
            .unwrap_or_else(|| PyKeyboardInterrupt::new_err("search interrupted"))),
        Err(error) => Err(error.into()),
    }
}
