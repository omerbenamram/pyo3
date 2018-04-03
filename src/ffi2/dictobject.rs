use std::os::raw::{c_char, c_int};
use ffi2::pyport::Py_ssize_t;
use ffi2::object::*;

//pub enum PyDictObject { /* representation hidden */ }

#[cfg_attr(windows, link(name="pythonXY"))] extern "C" {
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_Type")]
    pub static mut PyDict_Type: PyTypeObject;
    pub static mut PyDictIterKey_Type: PyTypeObject;
    pub static mut PyDictIterValue_Type: PyTypeObject;
    pub static mut PyDictIterItem_Type: PyTypeObject;
    pub static mut PyDictKeys_Type: PyTypeObject;
    pub static mut PyDictItems_Type: PyTypeObject;
    pub static mut PyDictValues_Type: PyTypeObject;
}

#[inline(always)]
pub unsafe fn PyDict_Check(op : *mut PyObject) -> c_int {
    PyType_FastSubclass(Py_TYPE(op), Py_TPFLAGS_DICT_SUBCLASS)
}

#[inline(always)]
pub unsafe fn PyDict_CheckExact(op : *mut PyObject) -> c_int {
    let u : *mut PyTypeObject = &mut PyDict_Type;
    (Py_TYPE(op) == u) as c_int
}

#[cfg_attr(windows, link(name="pythonXY"))] extern "C" {
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_New")]
    pub fn PyDict_New() -> *mut PyObject;
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDictProxy_New")]
    pub fn PyDictProxy_New(dict: *mut PyObject) -> *mut PyObject;
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_Clear")]
    pub fn PyDict_Clear(mp: *mut PyObject);
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_Contains")]
    pub fn PyDict_Contains(mp: *mut PyObject, key: *mut PyObject)
     -> c_int;
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_Copy")]
    pub fn PyDict_Copy(mp: *mut PyObject) -> *mut PyObject;
    
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_GetItem")]
    pub fn PyDict_GetItem(mp: *mut PyObject, key: *mut PyObject)
     -> *mut PyObject;
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_SetItem")]
    pub fn PyDict_SetItem(mp: *mut PyObject, key: *mut PyObject,
                          item: *mut PyObject) -> c_int;
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_DelItem")]
    pub fn PyDict_DelItem(mp: *mut PyObject, key: *mut PyObject)
     -> c_int;
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_GetItemString")]
    pub fn PyDict_GetItemString(dp: *mut PyObject, key: *const c_char)
     -> *mut PyObject;
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_SetItemString")]
    pub fn PyDict_SetItemString(dp: *mut PyObject, key: *const c_char,
                                item: *mut PyObject) -> c_int;
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_DelItemString")]
    pub fn PyDict_DelItemString(dp: *mut PyObject, key: *const c_char)
     -> c_int;
    
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_Keys")]
    pub fn PyDict_Keys(mp: *mut PyObject) -> *mut PyObject;
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_Values")]
    pub fn PyDict_Values(mp: *mut PyObject) -> *mut PyObject;
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_Items")]
    pub fn PyDict_Items(mp: *mut PyObject) -> *mut PyObject;
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_Size")]
    pub fn PyDict_Size(mp: *mut PyObject) -> Py_ssize_t;
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_Next")]
    pub fn PyDict_Next(mp: *mut PyObject, pos: *mut Py_ssize_t,
                       key: *mut *mut PyObject, value: *mut *mut PyObject)
     -> c_int;
    /*pub fn _PyDict_Next(mp: *mut PyObject, pos: *mut Py_ssize_t,
                        key: *mut *mut PyObject, value: *mut *mut PyObject,
                        hash: *mut c_long) -> c_int;
    pub fn _PyDict_Contains(mp: *mut PyObject, key: *mut PyObject,
                            hash: c_long) -> c_int;
    pub fn _PyDict_NewPresized(minused: Py_ssize_t) -> *mut PyObject;
    pub fn _PyDict_MaybeUntrack(mp: *mut PyObject);*/
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_Update")]
    pub fn PyDict_Update(mp: *mut PyObject, other: *mut PyObject)
     -> c_int;
    #[cfg_attr(PyPy, link_name="\u{1}_PyPyDict_Merge")]
    pub fn PyDict_Merge(mp: *mut PyObject, other: *mut PyObject,
                        _override: c_int) -> c_int;
    pub fn PyDict_MergeFromSeq2(d: *mut PyObject, seq2: *mut PyObject,
                                _override: c_int) -> c_int;

}