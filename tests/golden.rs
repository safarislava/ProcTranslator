use proc_translator::machine::simulation::{DeviceChoice, InterruptRequest, simulate_machine};
use proc_translator::translator::asm::translate;
use proc_translator::translator::common::compile_to_hir;
use proc_translator::translator::lir::compile_lir;
use serde::{Serialize, Serializer, ser::SerializeMap};
use std::fs;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing::subscriber::DefaultGuard;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub struct TestOutput {
    pub program: String,
    input: Vec<InterruptLog>,
    int_output: Vec<i64>,
    char_output: Vec<char>,
    pub log: String,
}

#[derive(Serialize)]
pub struct InterruptLog {
    tick: u64,
    value: i64,
}

impl Serialize for TestOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(0))?;
        map.serialize_entry("program", &self.program)?;
        map.serialize_entry("input", &self.input)?;
        map.serialize_entry("int_output", &self.int_output)?;
        map.serialize_entry("char_output", &self.char_output.iter().collect::<String>())?;
        map.serialize_entry("log", &self.log)?;
        map.end()
    }
}

struct TestWriter(Arc<Mutex<Vec<u8>>>);

impl<'a> fmt::MakeWriter<'a> for TestWriter {
    type Writer = TestWriterGuard<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        TestWriterGuard {
            inner: self.0.lock().unwrap(),
        }
    }
}

struct TestWriterGuard<'a> {
    inner: std::sync::MutexGuard<'a, Vec<u8>>,
}

impl<'a> Write for TestWriterGuard<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn setup_test_logger() -> (DefaultGuard, Arc<Mutex<Vec<u8>>>) {
    let buffer = Arc::new(Mutex::new(Vec::new()));

    let layer = fmt::layer()
        .with_ansi(false)
        .with_target(false)
        .without_time()
        .compact()
        .with_writer(TestWriter(buffer.clone()));

    let subscriber = tracing_subscriber::registry()
        .with(layer)
        .with(EnvFilter::new("debug"));

    let guard = tracing::subscriber::set_default(subscriber);
    (guard, buffer)
}

fn run_test(name: &str, interrupts: Vec<InterruptRequest>) -> TestOutput {
    let content = fs::read_to_string(format!("examples/correct/{name}.java"))
        .unwrap_or_else(|e| panic!("Failed to read examples/correct/{name}.java: {e}"));

    let (guard, log_buffer) = setup_test_logger();

    let control_flow_graph = compile_to_hir(&content).expect("HIR compilation failed");

    let lir_package = compile_lir(control_flow_graph);
    let package = translate(lir_package);
    let (int_output, char_output) = simulate_machine(package, interrupts.clone());

    drop(guard);

    let log: String = {
        let buf = log_buffer.lock().unwrap();
        let raw = String::from_utf8_lossy(&buf);
        raw.lines()
            .map(|line| line.trim_end().to_string())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    };

    let clean_program = content
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");

    TestOutput {
        program: clean_program,
        input: interrupts
            .iter()
            .map(|i| InterruptLog {
                tick: i.tick,
                value: i.value,
            })
            .collect(),
        int_output,
        char_output,
        log,
    }
}

#[macro_export]
macro_rules! assert_golden_yaml {
    ($output:expr, $snapshot_name:expr) => {{
        use insta::assert_snapshot;
        use serde_yaml;

        let yaml = serde_yaml::to_string($output).expect("Failed to serialize TestOutput to YAML");

        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_path("snapshots");
        settings.set_prepend_module_to_snapshot(false);

        settings.bind(|| {
            assert_snapshot!($snapshot_name, yaml);
        });
    }};
}

pub fn export_test_output(output: &TestOutput, path: &str) -> std::io::Result<()> {
    use serde_yaml;

    let yaml = serde_yaml::to_string(output).map_err(std::io::Error::other)?;

    fs::write(path, yaml)?;
    Ok(())
}

#[test]
fn test_classes() {
    let output = run_test("classes", vec![]);
    assert_golden_yaml!(&output, "classes");
}

#[test]
fn test_calc() {
    let output = run_test("calc", vec![]);
    assert_golden_yaml!(&output, "calc");
}

#[test]
fn test_return() {
    let output = run_test("return", vec![]);
    assert_golden_yaml!(&output, "return");
}

#[test]
fn test_while() {
    let output = run_test("while", vec![]);
    assert_golden_yaml!(&output, "while");
}

#[test]
fn test_for() {
    let output = run_test("for", vec![]);
    assert_golden_yaml!(&output, "for");
}

#[test]
fn test_bool() {
    let output = run_test("bool", vec![]);
    assert_golden_yaml!(&output, "bool");
}

#[test]
fn test_global() {
    let output = run_test("global", vec![]);
    assert_golden_yaml!(&output, "global");
}

#[test]
fn test_params() {
    let output = run_test("params", vec![]);
    assert_golden_yaml!(&output, "params");
}

#[test]
fn test_array() {
    let output = run_test("array", vec![]);
    assert_golden_yaml!(&output, "array");
}

#[test]
fn test_cat() {
    let output = run_test(
        "cat",
        vec![
            InterruptRequest {
                tick: 200,
                value: 72,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 250,
                value: 101,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 350,
                value: 108,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 450,
                value: 108,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 550,
                value: 111,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 650,
                value: 0,
                device: DeviceChoice::CharInput,
            },
        ],
    );
    assert_golden_yaml!(&output, "cat");
}

#[test]
fn test_hello_world() {
    let output = run_test("hello_world", vec![]);
    assert_golden_yaml!(&output, "hello world");
}

#[test]
fn test_hello_user() {
    let output = run_test(
        "hello_user",
        vec![
            InterruptRequest {
                tick: 200,
                value: 115,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 250,
                value: 97,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 300,
                value: 102,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 350,
                value: 97,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 400,
                value: 114,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 450,
                value: 105,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 500,
                value: 115,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 550,
                value: 108,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 600,
                value: 97,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 650,
                value: 118,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 700,
                value: 97,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 750,
                value: 0,
                device: DeviceChoice::CharInput,
            },
        ],
    );
    assert_golden_yaml!(&output, "hello_user");
}

#[test]
fn test_sort() {
    let output = run_test(
        "sort",
        vec![
            InterruptRequest {
                tick: 150,
                value: 6,
                device: DeviceChoice::IntInput,
            },
            InterruptRequest {
                tick: 200,
                value: 101,
                device: DeviceChoice::IntInput,
            },
            InterruptRequest {
                tick: 250,
                value: 3,
                device: DeviceChoice::IntInput,
            },
            InterruptRequest {
                tick: 300,
                value: -99,
                device: DeviceChoice::IntInput,
            },
            InterruptRequest {
                tick: 350,
                value: 99,
                device: DeviceChoice::IntInput,
            },
            InterruptRequest {
                tick: 400,
                value: 24,
                device: DeviceChoice::IntInput,
            },
            InterruptRequest {
                tick: 450,
                value: 52,
                device: DeviceChoice::IntInput,
            },
        ],
    );
    assert_golden_yaml!(&output, "sort");
}

#[test]
fn test_vector() {
    let output = run_test("vector", vec![]);
    assert_golden_yaml!(&output, "vector");
}

#[test]
fn test_double() {
    let output = run_test("double", vec![]);
    assert_golden_yaml!(&output, "double");
}

#[test]
fn test_bitwise() {
    let output = run_test("bitwise", vec![]);
    assert_golden_yaml!(&output, "bitwise");
}

#[test]
fn test_vector_test() {
    let output = run_test("vector_test", vec![]);
    assert_golden_yaml!(&output, "vector_test");
}

#[test]
fn test_vector_test_simd() {
    let output = run_test("vector_test_simd", vec![]);
    assert_golden_yaml!(&output, "vector_test_simd");
}
