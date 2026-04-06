use proc_translator::machine::simulation::{InterruptRequest, simulate_machine};
use proc_translator::translator::asm::translate;
use proc_translator::translator::common::compile_to_hir;
use proc_translator::translator::lir::compile_lir;
use serde::{Serialize, Serializer, ser::SerializeMap};
use std::fs;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing::subscriber::DefaultGuard;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[derive(Debug)]
pub struct TestOutput {
    pub in_source: String,
    pub out_log: String,
}

impl Serialize for TestOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("in_source", &self.in_source)?;
        map.serialize_entry("out_log", &self.out_log)?;
        map.end()
    }
}

struct TestWriter(Arc<Mutex<Vec<u8>>>);

impl<'a> fmt::MakeWriter<'a> for TestWriter {
    type Writer = TestWriterGuard<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        TestWriterGuard {
            inner: self.0.lock().unwrap(),
            needs_newline: false,
        }
    }
}

struct TestWriterGuard<'a> {
    inner: std::sync::MutexGuard<'a, Vec<u8>>,
    needs_newline: bool,
}

impl<'a> Write for TestWriterGuard<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.needs_newline && !buf.is_empty() {
            self.inner.push(b'\n');
        }
        self.inner.extend_from_slice(buf);
        self.needs_newline = !buf.ends_with(b"\n");
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

    let (text_section, data_section, interrupt_blocks) = compile_lir(control_flow_graph);
    let package = translate(text_section, data_section, interrupt_blocks);
    simulate_machine(package, interrupts);

    drop(guard);

    let out_log: String = {
        let buf = log_buffer.lock().unwrap();
        let raw = String::from_utf8_lossy(&buf);
        raw.lines()
            .map(|line| line.trim_end().to_string())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    };

    TestOutput {
        in_source: content,
        out_log,
    }
}

fn format_yaml_with_literal_blocks(yaml: &str) -> String {
    let mut result = String::new();
    let lines: Vec<&str> = yaml.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("out_log:") {
            result.push_str("out_log: |\n");
            i += 1;

            while i < lines.len() {
                let list_line = lines[i];

                if list_line.trim_start().starts_with("- ")
                    || list_line.trim_start().starts_with("-'")
                {
                    let trimmed = list_line.trim_start();
                    let content = if trimmed.starts_with("- '") && trimmed.ends_with('\'') {
                        &trimmed[3..trimmed.len() - 1]
                    } else if trimmed.starts_with("- ") {
                        &trimmed[2..]
                    } else {
                        trimmed
                    };
                    result.push_str(&format!("  {}\n", content));
                    i += 1;
                } else {
                    break;
                }
            }
        } else {
            result.push_str(line);
            result.push('\n');
            i += 1;
        }
    }

    result.trim_end().to_string()
}

#[macro_export]
macro_rules! assert_golden_yaml {
    ($output:expr, $snapshot_name:expr) => {{
        use insta::assert_snapshot;
        use serde_yaml;

        let yaml = serde_yaml::to_string($output).expect("Failed to serialize TestOutput to YAML");

        let formatted = $crate::format_yaml_with_literal_blocks(&yaml);

        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_path("snapshots");
        settings.set_prepend_module_to_snapshot(false);

        settings.bind(|| {
            assert_snapshot!($snapshot_name, formatted);
        });
    }};
}

pub fn export_test_output(output: &TestOutput, path: &str) -> std::io::Result<()> {
    use serde_yaml;

    let yaml = serde_yaml::to_string(output).map_err(std::io::Error::other)?;

    let formatted = format_yaml_with_literal_blocks(&yaml);
    fs::write(path, formatted)?;
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
fn test_interrupt() {
    let output = run_test(
        "interrupt",
        vec![InterruptRequest {
            tick: 63,
            value: 1,
            port: 0,
            vector_port: 1,
        }],
    );
    assert_golden_yaml!(&output, "interrupt");
}
