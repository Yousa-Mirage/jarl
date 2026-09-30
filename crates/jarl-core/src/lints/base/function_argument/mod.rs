pub(crate) mod function_argument;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    #[test]
    fn test_no_lint_function_argument() {
        for code in [
            "function() {}",
            "function(x, y) {}",
            "function(x, y = 1, z = NULL) {}",
            "function(x = 1, y = 2) {}",
            "function(x = 1, ...) {}",
            "function(..., y) {}",
            "function(x, ..., y) {}",
            "function(x, y, ..., z = 1) {}",
            "function(x = 1, ..., y = NULL) {}",
            r"\(x, y = 1) {}",
            "function(x = 1) { function(a, b) {} }",
            "fun(x = 1, y)",
        ] {
            expect_no_lint(code, "function_argument", None);
        }
    }

    #[test]
    fn test_lint_function_argument() {
        let code = [
            "function(x, y = 1, z, w = 2) {}",
            "function(x = NULL, y, z) {}",
            "function(x = 1, ..., y, z = 2) {}",
            r"\(x = 1, y) {}",
            "function(\n  x # comment\n  = calculate(),\n  y\n) {}",
            "function(x = 1, `not syntactic`, 数据) {}",
            "function(x = 1, y) { function(a = 1, b) {} }",
            "function(x = function(a = 1, b) {}, y = 2) {}",
            "function(x = 1, y) if (missing(y)) 2 else y",
        ]
        .join("\n");

        assert_snapshot!(format_diagnostics(&code, "function_argument", None));
    }
}
