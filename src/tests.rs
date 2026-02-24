#[cfg(test)]
mod tests {
    use crate::compile_file;
    use std::fs;
    use crate::common::BoxError;

    fn run_tests_in_dir(dir_path: &str, should_succeed: bool) -> Result<(), BoxError> {
        println!("Running tests in: {}", dir_path);

        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() || path.extension().and_then(|s| s.to_str()) != Some("java") {
                continue;
            }

            let path_str = path.to_str().expect("Path should be valid UTF-8");
            println!("  Testing file: {}", path_str);

            let result = compile_file(path_str);

            if should_succeed {
                assert!(result.is_ok(), "Expected '{}' to compile successfully, but it failed with: {:?}", path_str, result.err());
            } else {
                assert!(result.is_err(), "Expected '{}' to fail compilation, but it succeeded.", path_str);
            }
        }
        Ok(())
    }

    #[test]
    fn test_correct_examples() -> Result<(), BoxError> {
        run_tests_in_dir("examples/correct", true)
    }

    #[test]
    fn test_incorrect_examples() -> Result<(), BoxError> {
        run_tests_in_dir("examples/incorrect", false)
    }
}