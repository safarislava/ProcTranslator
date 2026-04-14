use crate::translator::common::ResBox;
use crate::translator::{analyzer, ast, hir, parser, simplifier};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[allow(dead_code)]
pub fn create_cfg_schemes() {
    create_cfg_scheme("array").unwrap();
    create_cfg_scheme("bitwise").unwrap();
    create_cfg_scheme("bool").unwrap();
    create_cfg_scheme("calc").unwrap();
    create_cfg_scheme("cat").unwrap();
    create_cfg_scheme("classes").unwrap();
    create_cfg_scheme("double").unwrap();
    create_cfg_scheme("for").unwrap();
    create_cfg_scheme("global").unwrap();
    create_cfg_scheme("hello_user").unwrap();
    create_cfg_scheme("hello_world").unwrap();
    create_cfg_scheme("params").unwrap();
    create_cfg_scheme("prob1").unwrap();
    create_cfg_scheme("return").unwrap();
    create_cfg_scheme("sort").unwrap();
    create_cfg_scheme("vector").unwrap();
    create_cfg_scheme("vector_test").unwrap();
    create_cfg_scheme("vector_test_simd").unwrap();
    create_cfg_scheme("while").unwrap();
}

pub fn create_cfg_scheme(name: &str) -> ResBox<()> {
    let content = fs::read_to_string(format!("examples/{name}.java"))?;
    let syntax_tree = parser::parse_syntax_tree(&content)?;
    let ast = ast::build_ast(syntax_tree)?;
    let simple_ast = simplifier::simplify(ast);
    let typed_ast = analyzer::semantic_analyze(simple_ast)?;
    let control_flow_graph = hir::compile_hir(typed_ast);
    dump_to_file(format!("schemes/{}.dot", name), control_flow_graph.to_dot())?;
    Ok(())
}

pub fn dump_to_file(path: impl AsRef<Path>, value: String) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(value.as_bytes())?;
    Ok(())
}
