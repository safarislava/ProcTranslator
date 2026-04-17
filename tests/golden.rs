use proc_translator::io::load_interrupts;
use proc_translator::machine::printers::disassemble::disassemble;
use proc_translator::machine::simulation::{InterruptRequest, simulate_machine};
use proc_translator::translator::common::compile;
use serde::{Serialize, Serializer, ser::SerializeMap};
use std::fs;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing::subscriber::DefaultGuard;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[derive(Clone)]
pub struct TestOutput {
    pub program: String,
    input: Vec<InterruptLog>,
    int_output: Vec<i64>,
    char_output: Vec<char>,
    pub log: String,
    pub data_section: Vec<i64>,
    pub machine_code: String,
    pub ticks: u64,
}

#[derive(Serialize, Clone)]
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
        map.serialize_entry("ticks", &self.ticks)?;
        map.serialize_entry("input", &self.input)?;
        map.serialize_entry("int_output", &self.int_output)?;
        map.serialize_entry("char_output", &self.char_output.iter().collect::<String>())?;
        map.serialize_entry("data_section", &self.data_section)?;
        map.serialize_entry("machine_code", &self.machine_code)?;
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
    let content = fs::read_to_string(format!("examples/{name}.java"))
        .unwrap_or_else(|e| panic!("Failed to read examples/{name}.java: {e}"));

    let (guard, log_buffer) = setup_test_logger();

    let package = compile(&content).expect("Failed to compile");

    let data_section: Vec<i64> = package.data.iter().map(|value| *value as i64).collect();

    let machine_code = disassemble(&package.program)
        .iter()
        .map(|line| line.trim_end().to_string())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let (int_output, char_output, ticks) = simulate_machine(package, interrupts.clone());

    println!("Test results: ");
    println!(
        "Input : {:?}",
        interrupts
            .iter()
            .map(|i| (i.tick, i.value))
            .collect::<Vec<_>>()
    );
    println!("Int output : {:?}", int_output);
    println!("Char output : {}", char_output.iter().collect::<String>());
    println!("Total ticks : {} ", ticks);

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
        data_section,
        machine_code,
        ticks,
    }
}

#[macro_export]
macro_rules! assert_golden_yaml {
    ($output:expr, $snapshot_name:expr) => {{ assert_golden_yaml!($output, $snapshot_name, false) }};

    ($output:expr, $snapshot_name:expr, $last_log_only:expr) => {{
        use insta::assert_snapshot;
        use serde_yaml;

        let mut output_for_snapshot = ($output).clone();
        if $last_log_only {
            output_for_snapshot.log = output_for_snapshot
                .log
                .lines()
                .filter(|line| !line.trim().is_empty())
                .last()
                .map(|s| s.trim_end().to_string())
                .unwrap_or_default();
        }
        let yaml = serde_yaml::to_string(&output_for_snapshot)
            .expect("Failed to serialize TestOutput to YAML");
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
    assert_golden_yaml!(&output, "classes", false);
}

#[test]
fn test_calc() {
    let output = run_test("calc", vec![]);
    assert_golden_yaml!(&output, "calc", false);
}

#[test]
fn test_return() {
    let output = run_test("return", vec![]);
    assert_golden_yaml!(&output, "return", false);
}

#[test]
fn test_while() {
    let output = run_test("while", vec![]);
    assert_golden_yaml!(&output, "while", false);
}

#[test]
fn test_for() {
    let output = run_test("for", vec![]);
    assert_golden_yaml!(&output, "for", false);
}

#[test]
fn test_bool() {
    let output = run_test("bool", vec![]);
    assert_golden_yaml!(&output, "bool", false);
}

#[test]
fn test_global() {
    let output = run_test("global", vec![]);
    assert_golden_yaml!(&output, "global", true);
}

#[test]
fn test_params() {
    let output = run_test("params", vec![]);
    assert_golden_yaml!(&output, "params", false);
}

#[test]
fn test_array() {
    let output = run_test("array", vec![]);
    assert_golden_yaml!(&output, "array", true);
}

#[test]
fn test_cat() {
    let output = run_test("cat", load_interrupts("cat").unwrap());
    assert_golden_yaml!(&output, "cat", true);
}

#[test]
fn test_hello_world() {
    let output = run_test("hello_world", vec![]);
    assert_golden_yaml!(&output, "hello world", false);
}

#[test]
fn test_hello_user() {
    let output = run_test("hello_user", load_interrupts("hello_user").unwrap());
    assert_golden_yaml!(&output, "hello_user", true);
}

#[test]
fn test_sort() {
    let output = run_test("sort", load_interrupts("sort").unwrap());
    assert_golden_yaml!(&output, "sort", true);
}

#[test]
fn test_vector() {
    let output = run_test("vector", vec![]);
    assert_golden_yaml!(&output, "vector", false);
}

#[test]
fn test_double() {
    let output = run_test("double", vec![]);
    assert_golden_yaml!(&output, "double", true);
}

#[test]
fn test_bitwise() {
    let output = run_test("bitwise", vec![]);
    assert_golden_yaml!(&output, "bitwise", false);
}

#[test]
fn test_vector_test() {
    let output = run_test("vector_test", vec![]);
    assert_golden_yaml!(&output, "vector_test", true);
}

#[test]
fn test_vector_test_simd() {
    let output = run_test("vector_test_simd", vec![]);
    assert_golden_yaml!(&output, "vector_test_simd", true);
}

#[test]
fn test_matrix() {
    let output = run_test("matrix", vec![]);
    assert_golden_yaml!(&output, "matrix", true);
}

#[test]
fn test_matrix_simd() {
    let output = run_test("matrix_simd", vec![]);
    assert_golden_yaml!(&output, "matrix_simd", true);
}
