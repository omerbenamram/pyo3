# PyPy Support

Using PyPy is supported via cpyext.

Support is only provided for building rust extension for code running under PyPy. This means PyPy **cannot** be called from rust via cpyext.
 
This is a limitation of cpyext and supported for embedding cpyext is not planned.

Compilation against PyPy is done by exporting the `PYTHON_SYS_EXECUTABLE` to a pypy binary.

For example, `PYTHON_SYS_EXECUTABLE="/path/to/pypy3" /path/to/pypy3 setup.py install`