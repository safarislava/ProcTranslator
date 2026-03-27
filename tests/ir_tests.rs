use insta::{Settings, assert_snapshot};
use proc_translator::translator::common::compile_to_hir;
use std::fs;

fn get_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../tests/snapshots");
    settings
}

#[test]
fn test_hir_snapshots() {
    let settings = get_settings();
    settings.bind(|| {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let pattern = format!("{}/examples/correct/*.java", manifest_dir);

        glob::glob(&pattern).unwrap().for_each(|entry| {
            let path = entry.unwrap();
            let content = fs::read_to_string(&path).unwrap();
            let result = compile_to_hir(&content);

            assert!(result.is_ok(), "File {:?} should compile", path);

            let (cfg, _) = result.unwrap();
            let snapshot_name = path.file_stem().unwrap().to_str().unwrap().to_string();
            assert_snapshot!(snapshot_name, format!("{:#?}", cfg));
        });
    });
}
