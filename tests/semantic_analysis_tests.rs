use insta::{Settings, assert_snapshot};
use proc_translator::translator::ast::build_ast;
use proc_translator::translator::parser::parse_syntax_tree;
use proc_translator::translator::{analyzer, simplifier};
use std::fs;

fn get_settings() -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../tests/snapshots");
    settings
}

#[test]
fn test_semantic_analysis_snapshots() {
    let settings = get_settings();
    settings.bind(|| {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let pattern = format!("{}/examples/correct/*.java", manifest_dir);

        glob::glob(&pattern).unwrap().for_each(|entry| {
            let path = entry.unwrap();
            let content = fs::read_to_string(&path).unwrap();

            let syntax_tree = parse_syntax_tree(&content).unwrap();
            let ast = build_ast(syntax_tree).unwrap();
            let simple_ast = simplifier::simplify(ast);
            let typed_ast = analyzer::semantic_analyze(simple_ast);

            match typed_ast {
                Ok(tree) => {
                    let snapshot_name = path.file_stem().unwrap().to_str().unwrap().to_string();
                    assert_snapshot!(snapshot_name, format!("{:#?}", tree));
                }
                Err(e) => {
                    panic!("File {:?} should pass semantic analysis: {}", path, e);
                }
            }
        });
    });
}
