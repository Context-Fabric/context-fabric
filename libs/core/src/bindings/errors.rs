//! Maps [`CfError`] to Python exceptions.

use pyo3::exceptions::{
    PyFileNotFoundError, PyKeyboardInterrupt, PyPermissionError, PyRuntimeError, PyValueError,
};
use pyo3::prelude::*;

use crate::error::CfError;

pyo3::create_exception!(
    cfabric._core,
    SearchTemplateError,
    PyValueError,
    "A search template that Text-Fabric reports as bad.\n\n\
     The message holds Text-Fabric's messages, one per line, such as\n\
     `line 0: Unknown object type: \"wordd\"`."
);

impl From<CfError> for PyErr {
    fn from(error: CfError) -> Self {
        match error {
            CfError::Io { ref source, .. } if source.kind() == std::io::ErrorKind::NotFound => {
                PyFileNotFoundError::new_err(error.to_string())
            }
            CfError::Template(_) => SearchTemplateError::new_err(error.to_string()),
            CfError::Interrupted => PyKeyboardInterrupt::new_err(error.to_string()),
            CfError::UnreplaceableCache { .. } => PyPermissionError::new_err(error.to_string()),
            CfError::Parse { .. }
            | CfError::IncompatibleCache { .. }
            | CfError::MissingFeature(_) => PyValueError::new_err(error.to_string()),
            CfError::Regex(_) | CfError::Json(_) | CfError::Io { .. } => {
                PyRuntimeError::new_err(error.to_string())
            }
        }
    }
}
