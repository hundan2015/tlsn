# tlsn-python

Python bindings for TLSNotary SDK.

## What is included

This package currently exposes:

- initialize
- compute_reveal
- Prover
- Verifier
- Method
- NetworkSetting
- ProverConfig
- VerifierConfig
- HttpRequest
- Reveal

Python package entrypoint: tlsn_python
Rust extension module: tlsn_python.tlsn_python

## Prerequisites

- Rust toolchain (cargo and rustc)
- Python >= 3.9
- uv (recommended)
- maturin (installed in virtual environment)

## Create environment (uv)

From this directory:

```powershell
uv venv .venv-uv
uv pip install --python .venv-uv/Scripts/python.exe maturin
```

If cargo is not in PATH on Windows PowerShell:

```powershell
$env:PATH = "$env:USERPROFILE/.cargo/bin;" + $env:PATH
```

Optional shorter cargo cache path for long dependency paths:

```powershell
$env:CARGO_HOME = "D:/c"
$env:RUSTUP_HOME = "$env:USERPROFILE/.rustup"
```

## Develop install

Install editable package into the uv environment:

```powershell
./.venv-uv/Scripts/maturin.exe develop
```

Or from workspace root:

```powershell
uv run maturin develop -m crates/tlsn-python/Cargo.toml --interpreter crates/tlsn-python/.venv-uv/Scripts/python.exe
```

## Build wheel

Build release wheel to dist directory:

```powershell
./.venv-uv/Scripts/maturin.exe build --release -o dist
```

Install built wheel:

```powershell
./.venv-uv/Scripts/python.exe -m pip install --force-reinstall dist/<wheel-file>.whl
```

## Quick smoke test

```python
import asyncio
import tlsn_python as tp

async def main():
    await tp.initialize(thread_count=1)

    out = tp.compute_reveal(
        sent=b"GET / HTTP/1.1\\r\\nHost: example.com\\r\\n\\r\\n",
        recv=b"HTTP/1.1 200 OK\\r\\nContent-Length: 0\\r\\n\\r\\n",
        handlers=[{"type": "SENT", "part": "ALL", "action": "REVEAL"}],
    )

    print(out.keys())

asyncio.run(main())
```

## Notes

- initialize is async and should be awaited.
- This first version uses built-in network channel style for protocol flow methods.
- If you see path-length related git checkout errors on Windows, enable long paths:

```powershell
git config --global core.longpaths true
```
