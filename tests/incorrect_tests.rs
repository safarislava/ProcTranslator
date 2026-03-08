use insta::{Settings, assert_snapshot};
use proc_translator::translator::common::compile_to_ir;
use std::fs;

fn get_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../tests/snapshots");
    settings
}

#[test]
fn test_incorrect_examples_snapshots() {
    let settings = get_settings();
    settings.bind(|| {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let pattern = format!("{}/examples/incorrect/*.java", manifest_dir);

        glob::glob(&pattern).unwrap().for_each(|entry| {
            let path = entry.unwrap();
            let content = fs::read_to_string(&path).unwrap();
            let result = compile_to_ir(&content);

            assert!(result.is_err(), "File {:?} should fail to compile", path);

            let error = result.unwrap_err().to_string();
            let snapshot_name = path.file_stem().unwrap().to_str().unwrap().to_string();
            assert_snapshot!(snapshot_name, error);
        });
    });
}
