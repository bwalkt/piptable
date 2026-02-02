# Contributing

## Python build notes (pyo3)
- The Rust build links against your active Python runtime. Python 3.12 can
  cause missing symbol errors with `pyo3` (e.g. `_PyList_GetItemRef`).
- Use Python 3.13 and set `PYO3_PYTHON` to the 3.13 binary, e.g. via `.env.local`:

```bash
PYO3_PYTHON=/usr/local/bin/python3.13
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
```
