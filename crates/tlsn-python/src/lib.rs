use std::sync::{Arc, Mutex};

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pythonize::{depythonize, pythonize};
use tlsn_sdk_core::{compute_reveal as core_compute_reveal, ProverConfig, SdkProver, SdkVerifier, VerifierConfig};
use tokio::net::TcpStream;
use tokio::sync::Mutex as AsyncMutex;
use tokio_util::compat::TokioAsyncReadCompatExt;

fn py_runtime_error(msg: impl ToString) -> PyErr {
    PyRuntimeError::new_err(msg.to_string())
}

fn py_value_error(msg: impl ToString) -> PyErr {
    PyValueError::new_err(msg.to_string())
}

#[derive(Clone)]
struct ProgressEmitter {
    callback: Arc<Mutex<Option<Py<PyAny>>>>,
}

impl ProgressEmitter {
    fn new() -> Self {
        Self {
            callback: Arc::new(Mutex::new(None)),
        }
    }

    fn set_callback(&self, callback: Option<Py<PyAny>>) {
        if let Ok(mut guard) = self.callback.lock() {
            *guard = callback;
        }
    }

    fn emit(&self, step: &str, progress: f64, message: &str) {
        if let Ok(guard) = self.callback.lock() {
            if let Some(cb) = guard.as_ref() {
                Python::with_gil(|py| {
                    let payload = pyo3::types::PyDict::new_bound(py);
                    let _ = payload.set_item("step", step);
                    let _ = payload.set_item("progress", progress);
                    let _ = payload.set_item("message", message);
                    let _ = payload.set_item("source", "python");
                    let callback = cb.clone_ref(py);
                    let _ = callback.call1(py, (payload,));
                });
            }
        }
    }
}

#[pyclass]
struct Prover {
    inner: Arc<AsyncMutex<SdkProver>>,
    progress: ProgressEmitter,
}

#[pymethods]
impl Prover {
    #[new]
    fn new(config: &Bound<'_, PyAny>) -> PyResult<Self> {
        let config: ProverConfig = depythonize(config)
            .map_err(|e| py_value_error(format!("invalid prover config: {e}")))?;

        let inner = SdkProver::new(config).map_err(py_value_error)?;

        Ok(Self {
            inner: Arc::new(AsyncMutex::new(inner)),
            progress: ProgressEmitter::new(),
        })
    }

    #[pyo3(signature = (callback=None))]
    fn set_progress_callback(&self, callback: Option<Py<PyAny>>) {
        self.progress.set_callback(callback);
    }

    fn setup<'py>(&self, py: Python<'py>, verifier_addr: String) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);
        let progress = self.progress.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            progress.emit("MPC_SETUP", 0.1, "Connecting to verifier...");
            let stream = TcpStream::connect(&verifier_addr)
                .await
                .map_err(py_runtime_error)?;

            let mut prover = inner.lock().await;
            prover
                .setup(stream.compat())
                .await
                .map_err(py_runtime_error)?;

            progress.emit("MPC_SETUP", 0.2, "MPC setup complete");
            Ok(())
        })
    }

    fn send_request<'py>(
        &self,
        py: Python<'py>,
        server_addr: String,
        request: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let req = depythonize::<tlsn_sdk_core::HttpRequest>(request)
            .map_err(|e| py_value_error(format!("invalid http request: {e}")))?;

        let inner = Arc::clone(&self.inner);
        let progress = self.progress.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            progress.emit(
                "CONNECTING_TO_SERVER",
                0.3,
                "Connecting to application server...",
            );

            let stream = TcpStream::connect(&server_addr)
                .await
                .map_err(py_runtime_error)?;

            progress.emit("SENDING_REQUEST", 0.4, "Sending request...");

            let mut prover = inner.lock().await;
            let response = prover
                .send_request(stream.compat(), req)
                .await
                .map_err(py_runtime_error)?;

            progress.emit("REQUEST_COMPLETE", 0.5, "Response received");

            Python::with_gil(|py| {
                let obj = pythonize(py, &response)
                    .map_err(|e| py_runtime_error(format!("python conversion failed: {e}")))?;
                Ok(obj.unbind())
            })
        })
    }

    fn transcript(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let prover = self
            .inner
            .try_lock()
            .map_err(|_| py_runtime_error("prover is busy in an async operation"))?;

        let transcript = prover.transcript().map_err(py_runtime_error)?;
        let obj = pythonize(py, &transcript)
            .map_err(|e| py_runtime_error(format!("python conversion failed: {e}")))?;
        Ok(obj.unbind())
    }

    fn reveal<'py>(&self, py: Python<'py>, reveal: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let reveal = depythonize::<tlsn_sdk_core::Reveal>(reveal)
            .map_err(|e| py_value_error(format!("invalid reveal object: {e}")))?;

        let inner = Arc::clone(&self.inner);
        let progress = self.progress.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            progress.emit("REVEAL", 0.7, "Proving and revealing data...");

            let mut prover = inner.lock().await;
            prover.reveal(reveal).await.map_err(py_runtime_error)?;

            progress.emit("FINALIZED", 0.95, "Protocol finalized");
            Ok(())
        })
    }
}

#[pyclass]
struct Verifier {
    inner: Arc<AsyncMutex<SdkVerifier>>,
}

#[pymethods]
impl Verifier {
    #[new]
    fn new(config: &Bound<'_, PyAny>) -> PyResult<Self> {
        let config: VerifierConfig = depythonize(config)
            .map_err(|e| py_value_error(format!("invalid verifier config: {e}")))?;

        Ok(Self {
            inner: Arc::new(AsyncMutex::new(SdkVerifier::new(config))),
        })
    }

    fn connect<'py>(&self, py: Python<'py>, prover_addr: String) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let stream = TcpStream::connect(&prover_addr)
                .await
                .map_err(py_runtime_error)?;

            let mut verifier = inner.lock().await;
            verifier.connect(stream.compat()).await.map_err(py_runtime_error)?;
            Ok(())
        })
    }

    fn verify<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = Arc::clone(&self.inner);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut verifier = inner.lock().await;
            let output = verifier.verify().await.map_err(py_runtime_error)?;

            Python::with_gil(|py| {
                let obj = pythonize(py, &output)
                    .map_err(|e| py_runtime_error(format!("python conversion failed: {e}")))?;
                Ok(obj.unbind())
            })
        })
    }
}

#[pyfunction]
#[pyo3(signature = (logging_config=None, thread_count=1))]
fn initialize<'py>(
    py: Python<'py>,
    logging_config: Option<&Bound<'py, PyAny>>,
    thread_count: usize,
) -> PyResult<Bound<'py, PyAny>> {
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let _ = logging_config;
        if thread_count == 0 {
            return Err(py_value_error("thread_count must be > 0"));
        }

        let builder = rayon::ThreadPoolBuilder::new().num_threads(thread_count);
        if let Err(err) = builder.build_global() {
            // Global pool can only be initialized once; treat subsequent calls as no-op.
            let msg = err.to_string();
            if !msg.contains("global thread pool has already been initialized") {
                return Err(py_runtime_error(msg));
            }
        }

        Ok(())
    })
}

#[pyfunction]
fn compute_reveal(py: Python<'_>, sent: Vec<u8>, recv: Vec<u8>, handlers: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let handlers = depythonize::<Vec<tlsn_sdk_core::Handler>>(handlers)
        .map_err(|e| py_value_error(format!("invalid handlers: {e}")))?;

    let output = core_compute_reveal(&sent, &recv, &handlers).map_err(py_runtime_error)?;
    let obj = pythonize(py, &output)
        .map_err(|e| py_runtime_error(format!("python conversion failed: {e}")))?;

    Ok(obj.unbind())
}

#[pymodule]
fn tlsn_python(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Prover>()?;
    m.add_class::<Verifier>()?;
    m.add_function(wrap_pyfunction!(initialize, m)?)?;
    m.add_function(wrap_pyfunction!(compute_reveal, m)?)?;
    Ok(())
}
