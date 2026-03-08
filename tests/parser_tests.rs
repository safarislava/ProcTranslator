use insta::{Settings, assert_snapshot};
use proc_translator::translator::parser::parse_syntax_tree;
use std::fs;

fn get_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../tests/snapshots");
    settings
}

#[test]
fn test_parser_snapshots() {
    let settings = get_settings();
    settings.bind(|| {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let pattern = format!("{}/examples/correct/*.java", manifest_dir);

        glob::glob(&pattern).unwrap().for_each(|entry| {
            let path = entry.unwrap();
            let content = fs::read_to_string(&path).unwrap();

            let syntax_tree = parse_syntax_tree(&content);

            match syntax_tree {
                Ok(tree) => {
                    let snapshot_name =
                        format!("parser@{}", path.file_stem().unwrap().to_str().unwrap());
                    assert_snapshot!(snapshot_name, format!("{:#?}", tree));
                }
                Err(e) => {
                    panic!("File {:?} should parse successfully: {}", path, e);
                }
            }
        });
    });
}
