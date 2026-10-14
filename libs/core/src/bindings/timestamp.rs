//! Text-Fabric compatible message channel (`TF.tmObj`).
//!
//! Mirrors `tf.core.timestamp.Timestamp`: leveled, indented, time-stamped
//! messages written to Python's `sys.stdout` / `sys.stderr`, filtered by a
//! verbosity mode (`silent`), with an optional message cache. The `Fabric`
//! object owns one instance and exposes its methods as `TF.info`,
//! `TF.warning`, `TF.error`, `TF.indent`, ... exactly like Text-Fabric.

use std::collections::HashMap;
use std::time::Instant;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyList, PyString};

/// Show all messages.
pub(crate) const VERBOSE: &str = "verbose";
/// Like `verbose`, but feature headers show descriptions only (the default).
pub(crate) const AUTO: &str = "auto";
/// Like `auto`, but suppress informational messages.
pub(crate) const TERSE: &str = "terse";
/// Like `terse`, but also suppress warnings.
pub(crate) const DEEP: &str = "deep";
/// The default verbosity (`tf.core.timestamp.SILENT_D`).
pub(crate) const SILENT_D: &str = AUTO;

const ONE_LEVEL: &str = "   |   ";

/// Normalize a `silent` argument like TF's `silentConvert`: `None` -> `auto`,
/// `False` -> `verbose`, `True` -> `deep`, a mode name -> itself; any other
/// value is rejected (TF silently coerces it to a bool, which is never useful).
pub(crate) fn silent_convert(value: Option<&Bound<'_, PyAny>>) -> PyResult<&'static str> {
    let Some(value) = value else {
        return Ok(SILENT_D);
    };
    if value.is_none() {
        return Ok(SILENT_D);
    }
    if let Ok(flag) = value.downcast::<PyBool>() {
        return Ok(if flag.is_true() { DEEP } else { VERBOSE });
    }
    if let Ok(text) = value.downcast::<PyString>() {
        return match text.to_str()? {
            VERBOSE => Ok(VERBOSE),
            AUTO => Ok(AUTO),
            TERSE => Ok(TERSE),
            DEEP => Ok(DEEP),
            other => Err(PyValueError::new_err(format!(
                "silent must be one of verbose, auto, terse, deep (or a bool/None); got {other:?}"
            ))),
        };
    }
    Err(PyValueError::new_err(
        "silent must be one of verbose, auto, terse, deep, a bool, or None",
    ))
}

/// A normalized `silent=` argument (see [`silent_convert`]).
#[derive(Clone, Copy)]
pub(crate) struct Silent(pub(crate) &'static str);

impl Silent {
    /// The default verbosity, `auto`.
    pub(crate) const DEFAULT: Self = Self(SILENT_D);
}

impl<'py> FromPyObject<'py> for Silent {
    fn extract_bound(value: &Bound<'py, PyAny>) -> PyResult<Self> {
        silent_convert(Some(value)).map(Self)
    }
}

/// Where a message goes: printed (`0`), cached and printed (`1`), cached only
/// (`-1`), or appended to a caller-supplied list.
pub(crate) enum CacheTarget<'py> {
    Mode(i64),
    List(Bound<'py, PyList>),
}

impl<'py> FromPyObject<'py> for CacheTarget<'py> {
    fn extract_bound(value: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(list) = value.downcast::<PyList>() {
            return Ok(Self::List(list.clone()));
        }
        Ok(Self::Mode(value.extract::<i64>()?))
    }
}

/// Default `cache` argument: print only.
pub(crate) const PRINT_ONLY: CacheTarget<'static> = CacheTarget::Mode(0);

/// Write `text` to Python's `sys.stdout` (or `sys.stderr`), like TF's `console`.
pub(crate) fn write_console(
    py: Python<'_>,
    error: bool,
    newline: bool,
    text: &str,
) -> PyResult<()> {
    let channel = py
        .import("sys")?
        .getattr(if error { "stderr" } else { "stdout" })?;
    channel.call_method1(
        "write",
        (format!("{text}{}", if newline { "\n" } else { "" }),),
    )?;
    channel.call_method0("flush")?;
    Ok(())
}

/// Leveled, time-stamped message channel compatible with TF's `Timestamp`.
#[pyclass(name = "Timestamp", module = "cfabric._core")]
pub(crate) struct PyTimestamp {
    silent: &'static str,
    was_silent: &'static str,
    level: usize,
    /// Start instant per indentation level, for the elapsed-time prefix.
    timestamps: HashMap<usize, Instant>,
    /// Cached messages: `(is_error, newline, rendered message)`.
    log: Vec<(bool, bool, String)>,
}

impl PyTimestamp {
    pub(crate) fn with_silent(silent: &'static str) -> Self {
        let mut timestamps = HashMap::new();
        timestamps.insert(0, Instant::now());
        Self {
            silent,
            was_silent: AUTO,
            level: 0,
            timestamps,
            log: Vec::new(),
        }
    }

    fn level_rep(&self) -> String {
        ONE_LEVEL.repeat(self.level)
    }

    fn elapsed(&mut self) -> String {
        let start = *self
            .timestamps
            .entry(self.level)
            .or_insert_with(Instant::now);
        let interval = start.elapsed().as_secs_f64();
        if interval < 10.0 {
            return format!("{interval:>5.2}s");
        }
        let seconds = interval.round() as u64;
        if seconds < 60 {
            return format!("{seconds:>2}s");
        }
        if seconds < 3600 {
            return format!("{:>2}m {:02}s", seconds / 60, seconds % 60);
        }
        format!(
            "{:>2}h {:02}m {:02}s",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    }

    /// Render and route one message (TF `raw_msg`).
    pub(crate) fn emit(
        &mut self,
        py: Python<'_>,
        msg: &Bound<'_, PyAny>,
        tm: bool,
        nl: bool,
        cache: CacheTarget<'_>,
        error: bool,
    ) -> PyResult<()> {
        let msg = match msg.downcast::<PyString>() {
            Ok(text) => text.to_str()?.to_string(),
            Err(_) => msg.repr()?.to_str()?.to_string(),
        };
        let level_rep = self.level_rep();
        let rendered = if tm {
            format!("{level_rep}{:>7} {msg}", self.elapsed())
        } else {
            format!("{level_rep}{msg}")
        }
        .replace('\n', &format!("\n{level_rep}"));
        match cache {
            CacheTarget::List(list) => list.append((error, nl, rendered))?,
            CacheTarget::Mode(mode) => {
                if mode != 0 {
                    self.log.push((error, nl, rendered.clone()));
                }
                if mode >= 0 {
                    write_console(py, error, nl, &rendered)?;
                }
            }
        }
        Ok(())
    }

    /// Emit a plain message regardless of verbosity.
    pub(crate) fn emit_text(&mut self, py: Python<'_>, msg: &str, error: bool) -> PyResult<()> {
        self.emit(
            py,
            PyString::new(py, msg).as_any(),
            true,
            true,
            PRINT_ONLY,
            error,
        )
    }

    /// Emit a warning string unless warnings are silenced.
    pub(crate) fn warn_str(&mut self, py: Python<'_>, msg: &str) -> PyResult<()> {
        if matches!(self.silent, VERBOSE | AUTO | TERSE) {
            self.emit_text(py, msg, false)?;
        }
        Ok(())
    }

    /// Emit an error string (errors are never silenced).
    pub(crate) fn error_str(&mut self, py: Python<'_>, msg: &str) -> PyResult<()> {
        self.emit_text(py, msg, true)
    }
}

#[pymethods]
impl PyTimestamp {
    /// Create a message channel.
    ///
    /// Args:
    ///     silent: Verbosity: `verbose`, `auto` (default), `terse`, or `deep`.
    ///     level: Initial indentation level.
    #[new]
    #[pyo3(signature = (silent=Silent::DEFAULT, level=None), text_signature = "(silent='auto', level=None)")]
    fn new(silent: Silent, level: Option<usize>) -> PyResult<Self> {
        let mut stamp = Self::with_silent(silent.0);
        stamp.level = level.unwrap_or(0);
        stamp.timestamps.insert(stamp.level, Instant::now());
        Ok(stamp)
    }

    /// Write a message regardless of verbosity (TF `raw_msg`).
    ///
    /// Args:
    ///     msg: The message; non-strings are rendered with `repr`.
    ///     tm: Prefix the elapsed time at the current level.
    ///     nl: Append a newline.
    ///     cache: `0` print only, `1` cache and print, `-1` cache only, or a
    ///         list to append `(error, nl, message)` tuples to.
    ///     error: Write to standard error instead of standard output.
    #[pyo3(signature = (msg, tm=true, nl=true, cache=PRINT_ONLY, error=false), text_signature = "($self, msg, tm=True, nl=True, cache=0, error=False)")]
    pub(crate) fn raw_msg(
        &mut self,
        py: Python<'_>,
        msg: &Bound<'_, PyAny>,
        tm: bool,
        nl: bool,
        cache: CacheTarget<'_>,
        error: bool,
    ) -> PyResult<()> {
        self.emit(py, msg, tm, nl, cache, error)
    }

    /// Clear the message cache.
    pub(crate) fn reset(&mut self) {
        self.log.clear();
    }

    /// Flush the message cache to the console, or return it as a string.
    ///
    /// Args:
    ///     _asString: Return the cached messages instead of printing them.
    #[allow(non_snake_case)]
    #[pyo3(signature = (_asString=false))]
    pub(crate) fn cache(&mut self, py: Python<'_>, _asString: bool) -> PyResult<Option<String>> {
        let log = std::mem::take(&mut self.log);
        if _asString {
            return Ok(Some(
                log.into_iter()
                    .map(|(_, nl, msg)| format!("{msg}{}", if nl { "\n" } else { "" }))
                    .collect(),
            ));
        }
        for (error, nl, msg) in log {
            write_console(py, error, nl, &msg)?;
        }
        Ok(None)
    }

    /// Show a debug message (only in `verbose` mode unless forced).
    #[pyo3(signature = (msg, tm=true, nl=true, cache=PRINT_ONLY, force=false), text_signature = "($self, msg, tm=True, nl=True, cache=0, force=False)")]
    pub(crate) fn debug(
        &mut self,
        py: Python<'_>,
        msg: &Bound<'_, PyAny>,
        tm: bool,
        nl: bool,
        cache: CacheTarget<'_>,
        force: bool,
    ) -> PyResult<()> {
        if force || self.silent == VERBOSE {
            self.raw_msg(py, msg, tm, nl, cache, false)?;
        }
        Ok(())
    }

    /// Show an informational message (in `verbose` and `auto` mode unless forced).
    #[pyo3(signature = (msg, tm=true, nl=true, cache=PRINT_ONLY, force=false), text_signature = "($self, msg, tm=True, nl=True, cache=0, force=False)")]
    pub(crate) fn info(
        &mut self,
        py: Python<'_>,
        msg: &Bound<'_, PyAny>,
        tm: bool,
        nl: bool,
        cache: CacheTarget<'_>,
        force: bool,
    ) -> PyResult<()> {
        if force || matches!(self.silent, VERBOSE | AUTO) {
            self.raw_msg(py, msg, tm, nl, cache, false)?;
        }
        Ok(())
    }

    /// Show a warning (suppressed only in `deep` mode unless forced).
    #[pyo3(signature = (msg, tm=true, nl=true, cache=PRINT_ONLY, force=false), text_signature = "($self, msg, tm=True, nl=True, cache=0, force=False)")]
    pub(crate) fn warning(
        &mut self,
        py: Python<'_>,
        msg: &Bound<'_, PyAny>,
        tm: bool,
        nl: bool,
        cache: CacheTarget<'_>,
        force: bool,
    ) -> PyResult<()> {
        if force || matches!(self.silent, VERBOSE | AUTO | TERSE) {
            self.raw_msg(py, msg, tm, nl, cache, false)?;
        }
        Ok(())
    }

    /// Show an error on standard error, irrespective of the verbosity.
    #[pyo3(signature = (msg, tm=true, nl=true, cache=PRINT_ONLY, force=true), text_signature = "($self, msg, tm=True, nl=True, cache=0, force=True)")]
    pub(crate) fn error(
        &mut self,
        py: Python<'_>,
        msg: &Bound<'_, PyAny>,
        tm: bool,
        nl: bool,
        cache: CacheTarget<'_>,
        force: bool,
    ) -> PyResult<()> {
        let _ = force;
        self.raw_msg(py, msg, tm, nl, cache, true)
    }

    /// Set the indentation level of subsequent messages.
    ///
    /// Args:
    ///     level: An int sets the level; `True` / `False` increase / decrease it
    ///         by one (not below zero); `None` resets it to zero.
    ///     reset: Restart the elapsed-time clock at the resulting level.
    #[pyo3(signature = (level=None, reset=false))]
    pub(crate) fn indent(&mut self, level: Option<&Bound<'_, PyAny>>, reset: bool) -> PyResult<()> {
        let current = self.level as i64;
        let next = match level {
            None => 0,
            Some(value) if value.is_none() => 0,
            Some(value) => match value.downcast::<PyBool>() {
                Ok(flag) if flag.is_true() => current + 1,
                Ok(_) => current - 1,
                Err(_) => value.extract::<i64>()?,
            },
        };
        self.level = next.max(0) as usize;
        if reset {
            self.timestamps.insert(self.level, Instant::now());
        }
        Ok(())
    }

    /// The current verbosity mode.
    #[allow(non_snake_case)]
    pub(crate) fn isSilent(&self) -> &'static str {
        self.silent
    }

    /// Set the verbosity mode (`verbose`, `auto`, `terse`, `deep`, a bool, or `None`).
    #[allow(non_snake_case)]
    #[pyo3(signature = (silent))]
    pub(crate) fn setSilent(&mut self, silent: Option<&Bound<'_, PyAny>>) -> PyResult<()> {
        self.silent = silent_convert(silent)?;
        Ok(())
    }

    /// Suppress informational messages (and warnings too if `deep`).
    #[allow(non_snake_case)]
    #[pyo3(signature = (deep=false))]
    pub(crate) fn silentOn(&mut self, deep: bool) {
        self.was_silent = if matches!(self.silent, VERBOSE | AUTO | TERSE) {
            self.silent
        } else {
            AUTO
        };
        self.silent = if deep { DEEP } else { TERSE };
    }

    /// Restore the verbosity that was in effect before `silentOn`.
    #[allow(non_snake_case)]
    pub(crate) fn silentOff(&mut self) {
        self.silent = self.was_silent;
    }
}
