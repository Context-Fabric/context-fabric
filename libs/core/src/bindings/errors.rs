use pyo3::exceptions::{PyException, PyFileNotFoundError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;

use crate::error::CfError;

impl From<CfError> for PyErr {
    fn from(error: CfError) -> Self {
        match error {
            CfError::Io { ref source, .. } if source.kind() == std::io::ErrorKind::NotFound => {
                PyFileNotFoundError::new_err(error.to_string())
            }
            CfError::Parse { .. } | CfError::InvalidQuery(_) | CfError::InvalidSpec(_) => {
                PyValueError::new_err(error.to_string())
            }
            CfError::MissingFeature(_) => PyException::new_err(error.to_string()),
            CfError::Regex(_) | CfError::Json(_) | CfError::Yaml(_) | CfError::Io { .. } => {
                PyRuntimeError::new_err(error.to_string())
            }
        }
    }
}
