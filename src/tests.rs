#[cfg(test)]
mod tests {
    use crate::common::RawAST;
    use crate::compile_to_ir;
    use insta::{Settings, assert_snapshot};
    use std::env;
    use std::fs;

    fn get_settings() -> Settings {
        let mut settings = Settings::clone_current();
        settings.set_snapshot_path("../tests/snapshots");
        settings
    }

    #[test]
    fn test_correct_examples_snapshots() {
        let settings = get_settings();
        settings.bind(|| {
            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            let pattern = format!("{}/examples/correct/*.java", manifest_dir);

            glob::glob(&pattern).unwrap().for_each(|entry| {
                let path = entry.unwrap();
                let content = fs::read_to_string(&path).unwrap();
                let result = compile_to_ir(&content);

                assert!(result.is_ok(), "File {:?} should compile", path);

                let cfg = result.unwrap();
                let snapshot_name = format!(
                    "correct_cfg@{}",
                    path.file_stem().unwrap().to_str().unwrap()
                );
                assert_snapshot!(snapshot_name, format!("{:#?}", cfg));
            });
        });
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
                let snapshot_name = format!(
                    "incorrect_error@{}",
                    path.file_stem().unwrap().to_str().unwrap()
                );
                assert_snapshot!(snapshot_name, error);
            });
        });
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

                let syntax_tree = crate::parser::parse_syntax_tree(&content);

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

    #[test]
    fn test_ast_snapshots() {
        let settings = get_settings();
        settings.bind(|| {
            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            let pattern = format!("{}/examples/correct/*.java", manifest_dir);

            glob::glob(&pattern).unwrap().for_each(|entry| {
                let path = entry.unwrap();
                let content = fs::read_to_string(&path).unwrap();

                let syntax_tree = crate::parser::parse_syntax_tree(&content).unwrap();
                let ast = crate::ast::build(syntax_tree);

                match ast {
                    Ok(tree) => {
                        let snapshot_name =
                            format!("ast@{}", path.file_stem().unwrap().to_str().unwrap());
                        assert_snapshot!(snapshot_name, format!("{:#?}", tree));
                    }
                    Err(e) => {
                        panic!("File {:?} should build AST successfully: {}", path, e);
                    }
                }
            });
        });
    }

    #[test]
    fn test_simplified_ast_snapshots() {
        let settings = get_settings();
        settings.bind(|| {
            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            let pattern = format!("{}/examples/correct/*.java", manifest_dir);

            glob::glob(&pattern).unwrap().for_each(|entry| {
                let path = entry.unwrap();
                let content = fs::read_to_string(&path).unwrap();

                let syntax_tree = crate::parser::parse_syntax_tree(&content).unwrap();
                let ast = crate::ast::build(syntax_tree).unwrap();
                let simple_ast: RawAST = crate::simplifier::simplify(ast);

                let snapshot_name = format!(
                    "simplified_ast@{}",
                    path.file_stem().unwrap().to_str().unwrap()
                );
                assert_snapshot!(snapshot_name, format!("{:#?}", simple_ast));
            });
        });
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

                let syntax_tree = crate::parser::parse_syntax_tree(&content).unwrap();
                let ast = crate::ast::build(syntax_tree).unwrap();
                let simple_ast = crate::simplifier::simplify(ast);
                let typed_ast = crate::analyzer::semantic_analyze(simple_ast);

                match typed_ast {
                    Ok(tree) => {
                        let snapshot_name =
                            format!("typed_ast@{}", path.file_stem().unwrap().to_str().unwrap());
                        assert_snapshot!(snapshot_name, format!("{:#?}", tree));
                    }
                    Err(e) => {
                        panic!("File {:?} should pass semantic analysis: {}", path, e);
                    }
                }
            });
        });
    }
}
