//! The `C` API: `Computeds`, each `Computed`, and the `levUp`/`levDown` views.

use std::sync::Arc;

use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple};

use crate::bindings::api_ref::ApiRef;
use crate::bindings::convert::{heading_to_py, node_number};
use crate::index::Boundary;
use crate::mapped::{ComputedFeatureData, MappedCompiledCorpus};

/// Names of the computed data blocks every corpus has under `C`, sorted like
/// TF's `Call()`; `structure` joins them when a structure is configured.
pub(crate) const COMPUTED_NAMES: [&str; 8] = [
    "boundary",
    "characters",
    "levDown",
    "levUp",
    "levels",
    "order",
    "rank",
    "sections",
];

/// Lazy sequence that backs `C.levUp.data` / `C.levDown.data`, indexed like
/// Text-Fabric's tuples: `C.levUp.data[n - 1]` are the embedders of node `n`
/// (every node), `C.levDown.data[n - maxSlot - 1]` the non-slot embeddees of
/// non-slot node `n`. Rows are read on demand from the `CFRLEVU1` / `CFRLEVD1`
/// mmap CSRs instead of materializing every node's tuple (1.4M on BHSA).
/// `row(n)` reads the row of node `n` directly.
#[pyclass(name = "LevView", module = "cfabric._core")]
pub(crate) struct PyLevView {
    corpus: Arc<MappedCompiledCorpus>,
    down: bool,
}

impl PyLevView {
    /// The node of the first row: 1 for `levUp`, `maxSlot + 1` for `levDown`.
    fn first_node(&self) -> PyResult<u32> {
        Ok(if self.down {
            self.corpus.max_slot()? + 1
        } else {
            1
        })
    }

    fn len(&self) -> PyResult<usize> {
        let first = self.first_node()?;
        Ok((self.corpus.max_node() + 1).saturating_sub(first) as usize)
    }

    fn node_row(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        let row = if self.down {
            self.corpus.lev_down_row(node)?
        } else {
            self.corpus.lev_up_row(node)?
        }
        .unwrap_or_default();
        Ok(PyTuple::new(py, row)?.into())
    }
}

#[pymethods]
impl PyLevView {
    fn __getitem__(&self, py: Python<'_>, index: isize) -> PyResult<PyObject> {
        let len = self.len()? as isize;
        let position = if index < 0 { index + len } else { index };
        if !(0..len).contains(&position) {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "level data index out of range",
            ));
        }
        self.node_row(py, self.first_node()? + position as u32)
    }

    /// Embedders (`levUp`) or embeddees (`levDown`) of node `node` as a tuple.
    fn row(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] node: u32,
    ) -> PyResult<PyObject> {
        if node < self.first_node()? || node > self.corpus.max_node() {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "node {node} has no level data row"
            )));
        }
        self.node_row(py, node)
    }

    fn __len__(&self) -> PyResult<usize> {
        self.len()
    }
}

/// A compiled section heading key as TF's key: the int for int-valued section
/// features (`None` for a node without a value), the string otherwise.
fn section_key_to_py(py: Python<'_>, heading: &str, is_int: bool) -> PyResult<PyObject> {
    if !is_int {
        return Ok(PyString::new(py, heading).into_any().unbind());
    }
    Ok(match heading.parse::<i64>() {
        Ok(value) => value.into_pyobject(py)?.into_any().unbind(),
        Err(_) if heading.is_empty() => py.None(),
        Err(_) => PyString::new(py, heading).into_any().unbind(),
    })
}

/// A block of precomputed data (TF `C.<name>`), in `data`.
#[pyclass(name = "Computed", module = "cfabric._core")]
pub(crate) struct PyComputed {
    /// The precomputed data.
    #[pyo3(get)]
    data: PyObject,
    api: ApiRef,
}

impl PyComputed {
    pub(crate) fn new(data: PyObject, api: ApiRef) -> Self {
        Self { data, api }
    }
}

#[pymethods]
impl PyComputed {
    /// The API (`Corpus`) this data belongs to.
    #[getter]
    fn api(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.api.get(py)
    }
}

/// The precomputed data of an API (TF `C`): `levels`, `order`, `rank`,
/// `boundary`, `levUp`, `levDown`, `sections`, `characters`.
#[pyclass(name = "Computeds", module = "cfabric._core")]
pub(crate) struct PyComputeds {
    corpus: Arc<MappedCompiledCorpus>,
    api: ApiRef,
}

impl PyComputeds {
    pub(crate) fn new(corpus: Arc<MappedCompiledCorpus>, api: ApiRef) -> Self {
        Self { corpus, api }
    }

    fn computed(&self, py: Python<'_>, data: PyObject) -> PyComputed {
        PyComputed::new(data, self.api.clone_ref(py))
    }

    fn boundary_to_py(py: Python<'_>, boundary: Boundary) -> PyResult<PyObject> {
        let first = boundary
            .first_slots
            .iter()
            .map(|nodes| PyTuple::new(py, nodes.iter().copied()).map(Into::into))
            .collect::<PyResult<Vec<PyObject>>>()?;
        let last = boundary
            .last_slots
            .iter()
            .map(|nodes| PyTuple::new(py, nodes.iter().copied()).map(Into::into))
            .collect::<PyResult<Vec<PyObject>>>()?;
        let row: Vec<PyObject> = vec![
            PyTuple::new(py, first)?.into(),
            PyTuple::new(py, last)?.into(),
        ];
        Ok(PyTuple::new(py, row)?.into())
    }

    pub(crate) fn get(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
        match name {
            "levels" => Ok(self.levels(py)?.into_pyobject(py)?.into()),
            "order" => Ok(self.order(py)?.into_pyobject(py)?.into()),
            "rank" => Ok(self.rank(py)?.into_pyobject(py)?.into()),
            "boundary" => Ok(self.boundary(py)?.into_pyobject(py)?.into()),
            "levUp" => Ok(self.levUp(py)?.into_pyobject(py)?.into()),
            "levDown" => Ok(self.levDown(py)?.into_pyobject(py)?.into()),
            "sections" => Ok(self.sections(py)?.into_pyobject(py)?.into()),
            "characters" => Ok(self.characters(py)?.into_pyobject(py)?.into()),
            "structure" => Ok(self.structure(py)?.into_pyobject(py)?.into()),
            _ => Err(PyAttributeError::new_err(format!(
                "no computed feature named {name}"
            ))),
        }
    }

    /// The computed features TF's `Call()` lists, sorted; `structure` only when
    /// a usable structure is configured.
    pub(crate) fn names(&self) -> PyResult<Vec<&'static str>> {
        let mut names = COMPUTED_NAMES.to_vec();
        if self.corpus.structure_data()?.is_some() {
            names.push("structure");
        }
        Ok(names)
    }

    /// TF `C.structure.data`: `(headingFromNode, nodeFromHeading, multiple, top,
    /// up, down)` (`tf/core/prepare.py::structure`).
    fn structure_to_py(
        &self,
        py: Python<'_>,
        data: &crate::index::StructureData,
    ) -> PyResult<PyObject> {
        let heading_from_node = PyDict::new(py);
        for (node, heading) in &data.heading_from_node {
            heading_from_node.set_item(node, heading_to_py(py, heading)?)?;
        }
        let node_from_heading = PyDict::new(py);
        for (heading, node) in &data.node_from_heading {
            node_from_heading.set_item(heading_to_py(py, heading)?, node)?;
        }
        let multiple = PyDict::new(py);
        for (heading, nodes) in &data.multiple {
            multiple.set_item(heading_to_py(py, heading)?, PyTuple::new(py, nodes)?)?;
        }
        let up = PyDict::new(py);
        for (node, parent) in &data.up {
            up.set_item(node, parent)?;
        }
        let down = PyDict::new(py);
        for (node, children) in &data.down {
            down.set_item(node, PyTuple::new(py, children)?)?;
        }
        let parts: Vec<PyObject> = vec![
            heading_from_node.into(),
            node_from_heading.into(),
            multiple.into(),
            PyTuple::new(py, &data.top)?.into(),
            up.into(),
            down.into(),
        ];
        Ok(PyTuple::new(py, parts)?.into())
    }

    fn lev_view(&self, py: Python<'_>, down: bool) -> PyResult<PyComputed> {
        let view = Py::new(
            py,
            PyLevView {
                corpus: Arc::clone(&self.corpus),
                down,
            },
        )?;
        Ok(self.computed(py, view.into_any()))
    }

    /// Per section level, whether its section feature is int-valued.
    fn section_key_types(&self) -> PyResult<Vec<bool>> {
        let metadata = self.corpus.metadata();
        Ok(crate::mapped::MappedSections::new(&self.corpus)?
            .section_features()
            .iter()
            .map(|name| {
                metadata
                    .node_feature(name)
                    .and_then(|feature| feature.value_type())
                    == Some("int")
            })
            .collect())
    }

    fn sections_to_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        // TF C.sections.data shape: dict(sec1=, sec2=, seqFromNode=, nodeFromSeq=).
        let data = PyDict::new(py);
        let Some(sections) = self.corpus.sections_data()? else {
            for key in ["sec1", "sec2", "seqFromNode", "nodeFromSeq"] {
                data.set_item(key, PyDict::new(py))?;
            }
            return Ok(data.into());
        };

        // The compiled section maps key headings by their string form; TF keys
        // them by the section feature's value (int for int features).
        let key_types = self.section_key_types()?;
        let key = |level: usize, heading: &str| -> PyResult<PyObject> {
            section_key_to_py(py, heading, key_types.get(level).copied().unwrap_or(false))
        };

        let sec1 = PyDict::new(py);
        for (node, inner) in &sections.sec1 {
            let inner_dict = PyDict::new(py);
            for (heading, target) in inner {
                inner_dict.set_item(key(1, heading)?, *target)?;
            }
            sec1.set_item(node, inner_dict)?;
        }

        let sec2 = PyDict::new(py);
        for (node, inner) in &sections.sec2 {
            let inner_dict = PyDict::new(py);
            for (heading1, inner2) in inner {
                let inner2_dict = PyDict::new(py);
                for (heading2, target) in inner2 {
                    inner2_dict.set_item(key(2, heading2)?, *target)?;
                }
                inner_dict.set_item(key(1, heading1)?, inner2_dict)?;
            }
            sec2.set_item(node, inner_dict)?;
        }

        let seq_from_node = PyDict::new(py);
        for (node, seq) in &sections.seq_from_node {
            seq_from_node.set_item(node, PyTuple::new(py, seq.iter().copied())?)?;
        }

        let node_from_seq = PyDict::new(py);
        for (seq, node) in &sections.node_from_seq {
            node_from_seq.set_item(PyTuple::new(py, seq.iter().copied())?, *node)?;
        }

        data.set_item("sec1", sec1)?;
        data.set_item("sec2", sec2)?;
        data.set_item("seqFromNode", seq_from_node)?;
        data.set_item("nodeFromSeq", node_from_seq)?;
        Ok(data.into())
    }

    fn characters_to_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        // TF C.characters.data shape: dict{format: sorted [(char, count)]}.
        //
        let result = PyDict::new(py);
        for (format_name, counts) in self.corpus.format_characters()? {
            let rows = counts
                .into_iter()
                .map(|(character, count)| (character.to_string(), count))
                .collect::<Vec<_>>();
            result.set_item(format_name, PyTuple::new(py, rows)?)?;
        }
        Ok(result.into())
    }
}

#[pymethods]
impl PyComputeds {
    /// Node types with their level and node interval: `((type, avgSlots, first, last), ...)`.
    #[getter]
    fn levels(&self, py: Python<'_>) -> PyResult<PyComputed> {
        let slot_type = self.corpus.slot_type()?;
        let rows = match self.corpus.computed_feature("levels")? {
            Some(ComputedFeatureData::Levels(rows)) => rows,
            _ => Vec::new(),
        }
        .into_iter()
        .map(|(node_type, level, first, last)| {
            // TF computes the average slot count of every type as a float, but
            // writes the slot type's row with the int `1`
            // (`tf/core/prepare.py::levels`).
            let level: PyObject = if node_type == slot_type {
                1_i64.into_pyobject(py)?.into_any().unbind()
            } else {
                level.into_pyobject(py)?.into_any().unbind()
            };
            let row: Vec<PyObject> = vec![
                PyString::new(py, &node_type).into(),
                level,
                first.into_pyobject(py)?.into(),
                last.into_pyobject(py)?.into(),
            ];
            PyTuple::new(py, row).map(Into::into)
        })
        .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(self.computed(py, PyTuple::new(py, rows)?.into()))
    }

    /// All nodes in canonical order.
    #[getter]
    fn order(&self, py: Python<'_>) -> PyResult<PyComputed> {
        Ok(self.computed(py, PyTuple::new(py, self.corpus.order()?)?.into()))
    }

    /// Position of each node in the canonical order (`rank[n - 1]`).
    #[getter]
    fn rank(&self, py: Python<'_>) -> PyResult<PyComputed> {
        Ok(self.computed(py, PyTuple::new(py, self.corpus.rank()?)?.into()))
    }

    /// Nodes starting and ending at each slot: `(firstSlots, lastSlots)`.
    #[getter]
    fn boundary(&self, py: Python<'_>) -> PyResult<PyComputed> {
        let boundary = match self.corpus.computed_feature("boundary")? {
            Some(ComputedFeatureData::Boundary(boundary)) => boundary,
            _ => Boundary {
                first_slots: Vec::new(),
                last_slots: Vec::new(),
            },
        };
        Ok(self.computed(py, Self::boundary_to_py(py, boundary)?))
    }

    /// Embedders of each node, as a `LevView` indexed by node id.
    #[getter]
    #[allow(non_snake_case)]
    fn levUp(&self, py: Python<'_>) -> PyResult<PyComputed> {
        self.lev_view(py, false)
    }

    /// Embeddees of each node, as a `LevView` indexed by node id.
    #[getter]
    #[allow(non_snake_case)]
    fn levDown(&self, py: Python<'_>) -> PyResult<PyComputed> {
        self.lev_view(py, true)
    }

    /// Section indexes: `{sec1, sec2, seqFromNode, nodeFromSeq}`.
    #[getter]
    fn sections(&self, py: Python<'_>) -> PyResult<PyComputed> {
        Ok(self.computed(py, self.sections_to_py(py)?))
    }

    /// Character frequencies per text format: `{format: ((char, count), ...)}`.
    #[getter]
    fn characters(&self, py: Python<'_>) -> PyResult<PyComputed> {
        Ok(self.computed(py, self.characters_to_py(py)?))
    }

    /// TF `C.structure`; absent (`AttributeError`) without a usable structure.
    #[getter]
    fn structure(&self, py: Python<'_>) -> PyResult<PyComputed> {
        let Some(data) = self.corpus.structure_data()? else {
            return Err(PyAttributeError::new_err(
                "no computed feature named structure: structure types are not configured",
            ));
        };
        Ok(self.computed(py, self.structure_to_py(py, data)?))
    }

    fn __dir__(&self) -> PyResult<Vec<&'static str>> {
        self.names()
    }

    fn __getattr__(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
        self.get(py, name)
    }
}
