use super::PyTypeRef;
use crate::{
    function::PosArgs,
    iterator,
    protocol::PyIter,
    slots::{IteratorIterable, SlotConstructor, SlotIterator},
    PyClassImpl, PyContext, PyObjectRef, PyRef, PyResult, PyValue, VirtualMachine,
};

/// map(func, *iterables) --> map object
///
/// Make an iterator that computes the function using arguments from
/// each of the iterables. Stops when the shortest iterable is exhausted.
#[pyclass(module = false, name = "map")]
#[derive(Debug)]
pub struct PyMap {
    mapper: PyObjectRef,
    iterators: Vec<PyIter>,
}

impl PyValue for PyMap {
    fn class(vm: &VirtualMachine) -> &PyTypeRef {
        &vm.ctx.types.map_type
    }
}

impl SlotConstructor for PyMap {
    type Args = (PyObjectRef, PosArgs<PyIter>);

    fn py_new(cls: PyTypeRef, (mapper, iterators): Self::Args, vm: &VirtualMachine) -> PyResult {
        let iterators = iterators.into_vec();
        PyMap { mapper, iterators }.into_pyresult_with_type(vm, cls)
    }
}

#[pyimpl(with(SlotIterator, SlotConstructor), flags(BASETYPE))]
impl PyMap {
    #[pymethod(magic)]
    fn length_hint(&self, vm: &VirtualMachine) -> PyResult<usize> {
        self.iterators.iter().try_fold(0, |prev, cur| {
            let cur = iterator::length_hint(vm, cur.as_object().clone())?.unwrap_or(0);
            let max = std::cmp::max(prev, cur);
            Ok(max)
        })
    }
}

impl IteratorIterable for PyMap {}
impl SlotIterator for PyMap {
    fn next(zelf: &PyRef<Self>, vm: &VirtualMachine) -> PyResult {
        let next_objs = zelf
            .iterators
            .iter()
            .map(|iterator| iterator.next(vm))
            .collect::<Result<Vec<_>, _>>()?;

        // the mapper itself can raise StopIteration which does stop the map iteration
        vm.invoke(&zelf.mapper, next_objs)
    }
}

pub fn init(context: &PyContext) {
    PyMap::extend_class(context, &context.types.map_type);
}
