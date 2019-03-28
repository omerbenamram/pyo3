// Copyright (c) 2017-present PyO3 Project and Contributors
//
// based on Daniel Grunwald's https://github.com/dgrunwald/rust-cpython

use crate::err::{PyDowncastError, PyErr, PyResult};
use crate::ffi;
use crate::gil::{self, GILGuard};
use crate::instance::AsPyRef;
use crate::object::PyObject;
use crate::type_object::{PyTypeInfo, PyTypeObject};
use crate::types::{PyAny, PyDict, PyModule, PyType};
use crate::AsPyPointer;
use crate::{IntoPyPointer, PyTryFrom};
use std::ffi::CString;
use std::marker::PhantomData;
use std::os::raw::c_int;
use std::ptr::NonNull;

pub use gil::prepare_freethreaded_python;

/// Marker type that indicates that the GIL is currently held.
///
/// The 'Python' struct is a zero-size marker struct that is required for most Python operations.
/// This is used to indicate that the operation accesses/modifies the Python interpreter state,
/// and thus can only be called if the Python interpreter is initialized and the
/// Python global interpreter lock (GIL) is acquired. The lifetime `'p` represents the lifetime of
/// the Python interpreter.
///
/// Note that the GIL can be temporarily released by the python interpreter during a function call
/// (e.g. importing a module), even when you're holding a GILGuard. In general, you don't need to
/// worry about this becauseas the GIL is reaquired before returning to the rust code:
///
/// ```text
/// GILGuard          |=====================================|
/// GIL actually held |==========|         |================|
/// Rust code running |=======|                |==|  |======|
/// ```
///
/// This behaviour can cause deadlocks when trying to lock while holding a GILGuard:
///
///  * Thread 1 acquires the GIL
///  * Thread 1 locks a mutex
///  * Thread 1 makes a call into the python interpreter, which releases the GIL
///  * Thread 2 acquires the GIL
///  * Thraed 2 tries to locks the mutex, blocks
///  * Thread 1's python interpreter call blocks trying to reacquire the GIL held by thread 2
///
/// To avoid deadlocking, you should release the GIL before trying to lock a mutex, e.g. with
/// [Python::allow_threads].
#[derive(Copy, Clone)]
pub struct Python<'p>(PhantomData<&'p GILGuard>);

impl<'p> Python<'p> {
    /// Retrieve Python instance under the assumption that the GIL is already acquired at this point,
    /// and stays acquired for the lifetime `'p`.
    ///
    /// Because the output lifetime `'p` is not connected to any input parameter,
    /// care must be taken that the compiler infers an appropriate lifetime for `'p`
    /// when calling this function.
    #[inline]
    pub unsafe fn assume_gil_acquired() -> Python<'p> {
        Python(PhantomData)
    }

    /// Acquires the global interpreter lock, which allows access to the Python runtime.
    ///
    /// If the Python runtime is not already initialized, this function will initialize it.
    /// See [prepare_freethreaded_python()](fn.prepare_freethreaded_python.html) for details.
    #[inline]
    pub fn acquire_gil() -> GILGuard {
        GILGuard::acquire()
    }

    /// Temporarily releases the `GIL`, thus allowing other Python threads to run.
    pub fn allow_threads<T, F>(self, f: F) -> T
    where
        F: Send + FnOnce() -> T,
    {
        // The `Send` bound on the closure prevents the user from
        // transferring the `Python` token into the closure.
        unsafe {
            let save = ffi::PyEval_SaveThread();
            let result = f();
            ffi::PyEval_RestoreThread(save);
            result
        }
    }

    /// Evaluates a Python expression in the given context and returns the result.
    ///
    /// If `globals` is `None`, it defaults to Python module `__main__`.
    /// If `locals` is `None`, it defaults to the value of `globals`.
    pub fn eval(
        self,
        code: &str,
        globals: Option<&PyDict>,
        locals: Option<&PyDict>,
    ) -> PyResult<&'p PyAny> {
        self.run_code(code, ffi::Py_eval_input, globals, locals)
    }

    /// Executes one or more Python statements in the given context.
    ///
    /// If `globals` is `None`, it defaults to Python module `__main__`.
    /// If `locals` is `None`, it defaults to the value of `globals`.
    pub fn run(
        self,
        code: &str,
        globals: Option<&PyDict>,
        locals: Option<&PyDict>,
    ) -> PyResult<&'p PyAny> {
        self.run_code(code, ffi::Py_file_input, globals, locals)
    }

    /// Runs code in the given context.
    /// `start` indicates the type of input expected:
    /// one of `Py_single_input`, `Py_file_input`, or `Py_eval_input`.
    ///
    /// If `globals` is `None`, it defaults to Python module `__main__`.
    /// If `locals` is `None`, it defaults to the value of `globals`.
    fn run_code(
        self,
        code: &str,
        start: c_int,
        globals: Option<&PyDict>,
        locals: Option<&PyDict>,
    ) -> PyResult<&'p PyAny> {
        let code = CString::new(code)?;

        unsafe {
            let mptr = ffi::PyImport_AddModule("__main__\0".as_ptr() as *const _);
            if mptr.is_null() {
                return Err(PyErr::fetch(self));
            }

            let globals = globals
                .map(AsPyPointer::as_ptr)
                .unwrap_or_else(|| ffi::PyModule_GetDict(mptr));
            let locals = locals.map(AsPyPointer::as_ptr).unwrap_or(globals);

            let res_ptr = ffi::PyRun_StringFlags(
                code.as_ptr(),
                start,
                globals,
                locals,
                ::std::ptr::null_mut(),
            );

            self.from_owned_ptr_or_err(res_ptr)
        }
    }

    /// Gets the Python type object for type `T`.
    pub fn get_type<T>(self) -> &'p PyType
    where
        T: PyTypeObject,
    {
        unsafe { self.from_borrowed_ptr(T::type_object().into_ptr()) }
    }

    /// Import the Python module with the specified name.
    pub fn import(self, name: &str) -> PyResult<&'p PyModule> {
        PyModule::import(self, name)
    }

    /// Check whether `obj` is an instance of type `T` like Python `isinstance` function
    pub fn is_instance<T: PyTypeObject, V: AsPyPointer>(self, obj: &V) -> PyResult<bool> {
        T::type_object().as_ref(self).is_instance(obj)
    }

    /// Check whether type `T` is subclass of type `U` like Python `issubclass` function
    pub fn is_subclass<T, U>(self) -> PyResult<bool>
    where
        T: PyTypeObject,
        U: PyTypeObject,
    {
        T::type_object().as_ref(self).is_subclass::<U>()
    }

    /// Gets the Python builtin value `None`.
    #[allow(non_snake_case)] // the Python keyword starts with uppercase
    #[inline]
    pub fn None(self) -> PyObject {
        unsafe { PyObject::from_borrowed_ptr(self, ffi::Py_None()) }
    }

    /// Gets the Python builtin value `NotImplemented`.
    #[allow(non_snake_case)] // the Python keyword starts with uppercase
    #[inline]
    pub fn NotImplemented(self) -> PyObject {
        unsafe { PyObject::from_borrowed_ptr(self, ffi::Py_NotImplemented()) }
    }
}

impl<'p> Python<'p> {
    unsafe fn unchecked_downcast<T: PyTypeInfo>(self, ob: &PyAny) -> &'p T {
        if T::OFFSET == 0 {
            &*(ob as *const _ as *const T)
        } else {
            let ptr = (ob.as_ptr() as *mut u8).offset(T::OFFSET) as *mut T;
            &*ptr
        }
    }

    #[allow(clippy::cast_ref_to_mut)] // FIXME
    unsafe fn unchecked_mut_downcast<T: PyTypeInfo>(self, ob: &PyAny) -> &'p mut T {
        if T::OFFSET == 0 {
            &mut *(ob as *const _ as *mut T)
        } else {
            let ptr = (ob.as_ptr() as *mut u8).offset(T::OFFSET) as *mut T;
            &mut *ptr
        }
    }

    /// Register object in release pool, and try to downcast to specific type.
    pub fn checked_cast_as<T>(self, obj: PyObject) -> Result<&'p T, PyDowncastError>
    where
        T: PyTypeInfo,
    {
        let p = unsafe { gil::register_owned(self, obj.into_nonnull()) };
        <T as PyTryFrom>::try_from(p)
    }

    /// Register object in release pool, and do unchecked downcast to specific type.
    pub unsafe fn cast_as<T>(self, obj: PyObject) -> &'p T
    where
        T: PyTypeInfo,
    {
        let p = gil::register_owned(self, obj.into_nonnull());
        self.unchecked_downcast(p)
    }

    /// Register `ffi::PyObject` pointer in release pool
    pub unsafe fn from_borrowed_ptr_to_obj(self, ptr: *mut ffi::PyObject) -> &'p PyAny {
        match NonNull::new(ptr) {
            Some(p) => gil::register_borrowed(self, p),
            None => crate::err::panic_after_error(),
        }
    }

    /// Register `ffi::PyObject` pointer in release pool,
    /// and do unchecked downcast to specific type.

    pub unsafe fn from_owned_ptr<T>(self, ptr: *mut ffi::PyObject) -> &'p T
    where
        T: PyTypeInfo,
    {
        match NonNull::new(ptr) {
            Some(p) => {
                let p = gil::register_owned(self, p);
                self.unchecked_downcast(p)
            }
            None => crate::err::panic_after_error(),
        }
    }

    /// Register `ffi::PyObject` pointer in release pool,
    /// Do unchecked downcast to specific type. Returns mutable reference.
    pub unsafe fn mut_from_owned_ptr<T>(self, ptr: *mut ffi::PyObject) -> &'p mut T
    where
        T: PyTypeInfo,
    {
        match NonNull::new(ptr) {
            Some(p) => {
                let p = gil::register_owned(self, p);
                self.unchecked_mut_downcast(p)
            }
            None => crate::err::panic_after_error(),
        }
    }

    /// Register owned `ffi::PyObject` pointer in release pool.
    /// Returns `Err(PyErr)` if the pointer is `null`.
    /// do unchecked downcast to specific type.
    pub unsafe fn from_owned_ptr_or_err<T>(self, ptr: *mut ffi::PyObject) -> PyResult<&'p T>
    where
        T: PyTypeInfo,
    {
        match NonNull::new(ptr) {
            Some(p) => {
                let p = gil::register_owned(self, p);
                Ok(self.unchecked_downcast(p))
            }
            None => Err(PyErr::fetch(self)),
        }
    }

    /// Register owned `ffi::PyObject` pointer in release pool.
    /// Returns `None` if the pointer is `null`.
    /// do unchecked downcast to specific type.
    pub unsafe fn from_owned_ptr_or_opt<T>(self, ptr: *mut ffi::PyObject) -> Option<&'p T>
    where
        T: PyTypeInfo,
    {
        NonNull::new(ptr).map(|p| {
            let p = gil::register_owned(self, p);
            self.unchecked_downcast(p)
        })
    }

    /// Register borrowed `ffi::PyObject` pointer in release pool.
    /// Panics if the pointer is `null`.
    /// do unchecked downcast to specific type.
    pub unsafe fn from_borrowed_ptr<T>(self, ptr: *mut ffi::PyObject) -> &'p T
    where
        T: PyTypeInfo,
    {
        match NonNull::new(ptr) {
            Some(p) => {
                let p = gil::register_borrowed(self, p);
                self.unchecked_downcast(p)
            }
            None => crate::err::panic_after_error(),
        }
    }

    /// Register borrowed `ffi::PyObject` pointer in release pool.
    /// Panics if the pointer is `null`.
    /// do unchecked downcast to specific type.
    pub unsafe fn mut_from_borrowed_ptr<T>(self, ptr: *mut ffi::PyObject) -> &'p mut T
    where
        T: PyTypeInfo,
    {
        match NonNull::new(ptr) {
            Some(p) => {
                let p = gil::register_borrowed(self, p);
                self.unchecked_mut_downcast(p)
            }
            None => crate::err::panic_after_error(),
        }
    }

    /// Register borrowed `ffi::PyObject` pointer in release pool.
    /// Returns `Err(PyErr)` if the pointer is `null`.
    /// do unchecked downcast to specific type.
    pub unsafe fn from_borrowed_ptr_or_err<T>(self, ptr: *mut ffi::PyObject) -> PyResult<&'p T>
    where
        T: PyTypeInfo,
    {
        match NonNull::new(ptr) {
            Some(p) => {
                let p = gil::register_borrowed(self, p);
                Ok(self.unchecked_downcast(p))
            }
            None => Err(PyErr::fetch(self)),
        }
    }

    /// Register borrowed `ffi::PyObject` pointer in release pool.
    /// Returns `None` if the pointer is `null`.
    /// do unchecked downcast to specific `T`.
    pub unsafe fn from_borrowed_ptr_or_opt<T>(self, ptr: *mut ffi::PyObject) -> Option<&'p T>
    where
        T: PyTypeInfo,
    {
        NonNull::new(ptr).map(|p| {
            let p = gil::register_borrowed(self, p);
            self.unchecked_downcast(p)
        })
    }

    #[doc(hidden)]
    /// Pass value ownership to `Python` object and get reference back.
    /// Value get cleaned up on the GIL release.
    pub fn register_any<T: 'static>(self, ob: T) -> &'p T {
        unsafe { gil::register_any(ob) }
    }

    /// Release PyObject reference.
    #[inline]
    pub fn release<T>(self, ob: T)
    where
        T: IntoPyPointer,
    {
        unsafe {
            let ptr = ob.into_ptr();
            if !ptr.is_null() {
                ffi::Py_DECREF(ptr);
            }
        }
    }

    /// Release `ffi::PyObject` pointer.
    #[inline]
    pub fn xdecref<T: IntoPyPointer>(self, ptr: T) {
        unsafe { ffi::Py_XDECREF(ptr.into_ptr()) };
    }
}

#[cfg(test)]
mod test {
    use crate::objectprotocol::ObjectProtocol;
    use crate::types::{IntoPyDict, PyAny, PyBool, PyInt, PyList};
    use crate::Python;

    #[test]
    fn test_eval() {
        let gil = Python::acquire_gil();
        let py = gil.python();

        // Make sure builtin names are accessible
        let v: i32 = py
            .eval("min(1, 2)", None, None)
            .map_err(|e| e.print(py))
            .unwrap()
            .extract()
            .unwrap();
        assert_eq!(v, 1);

        let d = [("foo", 13)].into_py_dict(py);

        // Inject our own global namespace
        let v: i32 = py
            .eval("foo + 29", Some(d), None)
            .unwrap()
            .extract()
            .unwrap();
        assert_eq!(v, 42);

        // Inject our own local namespace
        let v: i32 = py
            .eval("foo + 29", None, Some(d))
            .unwrap()
            .extract()
            .unwrap();
        assert_eq!(v, 42);

        // Make sure builtin names are still accessible when using a local namespace
        let v: i32 = py
            .eval("min(foo, 2)", None, Some(d))
            .unwrap()
            .extract()
            .unwrap();
        assert_eq!(v, 2);
    }

    #[test]
    fn test_is_instance() {
        let gil = Python::acquire_gil();
        let py = gil.python();
        assert!(py
            .is_instance::<PyBool, PyAny>(PyBool::new(py, true).into())
            .unwrap());
        let list = PyList::new(py, &[1, 2, 3, 4]);
        assert!(!py.is_instance::<PyBool, _>(list.as_ref()).unwrap());
        assert!(py.is_instance::<PyList, _>(list.as_ref()).unwrap());
    }

    #[test]
    fn test_is_subclass() {
        let gil = Python::acquire_gil();
        let py = gil.python();
        assert!(py.is_subclass::<PyBool, PyInt>().unwrap());
        assert!(!py.is_subclass::<PyBool, PyList>().unwrap());
    }
}
