use pyo3::exceptions::PyValueError;
use pyo3::types::{PyDict, PyDictMethods, PyTuple};
use pyo3::{Bound, PyResult, Python};

use crate::array::Array;

use pyo3::buffer::{Element, PyBuffer};
use pyo3::prelude::*;

use crate::buffer::{Buffer, SharedStorage};

fn import_contiguous<T: Element + Copy + Send + 'static>(
    obj: Bound<'_, PyAny>,
) -> PyResult<Buffer<T>> {
    let buf = PyBuffer::<T>::get(&obj)?;

    if buf.dimensions() != 1 {
        return Err(PyValueError::new_err("expected a 1-D array"));
    }
    if !buf.is_c_contiguous() {
        return Err(PyValueError::new_err(
            "expected a contiguous array; pass np.ascontiguousarray(...)",
        ));
    }

    let ptr = buf.buf_ptr() as *mut T;
    let len = buf.item_count();

    if !(ptr as usize).is_multiple_of(std::mem::align_of::<T>()) {
        return Err(PyValueError::new_err("buffer is not aligned for T"));
    }

    let storage = unsafe { SharedStorage::from_foreign(ptr, len, Box::new(buf)) };
    Ok(Buffer::from_storage(storage))
}

pub fn to_array_interface<'py>(py: Python<'py>, arr: &Array) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("version", 3)?;
    d.set_item("shape", PyTuple::new(py, [arr.len()])?)?;
    d.set_item(
        "typestr",
        arr.ty()
            .data
            .type_str()
            .ok_or_else(|| PyValueError::new_err("cannot export to numpy"))?,
    )?;
    d.set_item(
        "data",
        PyTuple::new(py, [arr.as_byte_ptr() as usize, 0_usize])?,
    )?;
    d.set_item("strides", py.None())?;
    Ok(d)
}

pub fn from_array_interface<'py>(py: Python<'py>, obj: Bound<'_, PyAny>) -> PyResult<Array> {
    // Choose variant based on typestr
    let ai = obj.getattr("__array_interface__")?;
    let ai = ai.extract::<Py<PyDict>>()?;
    let ai = ai.bind(py);
    let typestr = ai
        .get_item("typestr")?
        .ok_or_else(|| PyValueError::new_err("missing typestr"))?
        .extract::<String>()?;

    match &typestr[1..] {
        "f8" => Ok(Array::Floats(import_contiguous::<f64>(obj)?)),
        "i8" => Ok(Array::Ints(import_contiguous::<i64>(obj)?)),
        "u8" => Ok(Array::UInts(import_contiguous::<u64>(obj)?)),
        other => Err(PyValueError::new_err(format!(
            "unsupported dtype {other:?}; expected float64/int64/uint64"
        ))),
    }
}
