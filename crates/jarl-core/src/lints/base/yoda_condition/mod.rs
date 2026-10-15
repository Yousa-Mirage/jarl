pub(crate) mod yoda_condition;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "yoda_condition", None)
    }

    #[test]
    fn test_no_lint_yoda_condition() {
        for code in [
            "x == 1",
            "x != 'a'",
            "x < 2",
            "x <= 2",
            "x > 2",
            "x >= 2",
            "x == y",
            "1 + x == y",
            "NULL == x",
            "T == x",
            "1 %in% x",
            "1 + x",
            "expect_equal(foo(x), 2)",
            "expect_identical(x, 'a')",
            "expect_setequal(x, 1L)",
            "expect_equal(x, y)",
            "expect_equal(x$\"key\", 2)",
            "expect_equal(x@\"key\", 2)",
            "expect_equal(c(1, 2), x)",
            "expect_equal(1:3, x)",
            "expect_equal(1 * 2, x)",
            "expect_equal(1 + 2 + 3, x)",
            "expect_equal(1 + x, x)",
            "expect_equal((x) + 1, x)",
            "expect_equal(-x, x)",
            "expect_equal(!TRUE, x)",
            "expect_equal(-'a', x)",
            "expect_equal(NULL, x)",
            "expect_equal(T, x)",
            "expect_gt(2, x)",
            "other::expect_equal(2, x)",
            "x$expect_equal(2, x)",
            // Named arguments are matched before positional arguments.
            "expect_equal(expected = 2, object = x)",
            "expect_equal(expected = 2, x)",
            "expect_equal(`object` = x, 2)",
            "expect_equal('expected' = 2, x)",
            // Pipe input supplies the actual result.
            "x |> expect_equal(expected = 2, object = _)",
            "x %>% (expect_equal(2, .))",
            // Missing, forwarded, or incomplete arguments.
            "expect_equal(1)",
            "expect_equal(, x)",
            "expect_equal(object = 1, expected =)",
            "expect_equal(object = 1, object = x)",
            "expect_equal(..., 1, x)",
            "expect_equal(1, x",
        ] {
            expect_no_lint(code, "yoda_condition", None);
        }
        for pipe in ["|>", "%>%", "%!>%", "%T>%", "%<>%"] {
            expect_no_lint(
                &format!("x {pipe} expect_equal(1, 1)"),
                "yoda_condition",
                None,
            );
        }
    }

    #[test]
    fn test_lint_yoda_condition() {
        // Comparisons and expectations share literal detection.
        for literal in [
            "1",
            "1L",
            "1i",
            "'a'",
            "r\"(a\\b)\"",
            "TRUE",
            "FALSE",
            "NA",
            "NA_integer_",
            "Inf",
            "NaN",
            "-1",
            "((1))",
            "1 + 1",
            "(1) + 1",
            "2 + 1i",
        ] {
            for (code, replacement) in [
                (
                    format!("expect_equal({literal}, foo(x))"),
                    format!("expect_equal(foo(x), {literal})"),
                ),
                (
                    format!("{literal} == foo(x)"),
                    format!("foo(x) == {literal}"),
                ),
            ] {
                let diagnostics = check_code(&code, "yoda_condition", None);
                assert_eq!(diagnostics.len(), 1, "{code}");
                assert_eq!(
                    diagnostics[0].message.suggestion.as_deref(),
                    Some(format!("Use `{replacement}` instead.").as_str()),
                    "{code}"
                );
                assert!(diagnostics[0].has_unsafe_fix(), "{code}");
            }
        }

        // Ordinary fixes must leave comparisons and expectations unchanged.
        let code = "1 == total\nexpect_equal(42, total)";
        assert_eq!(
            get_fixed_text(vec![code], "yoda_condition", None),
            format!("OLD:\n====\n{code}\nNEW:\n====\n{code}")
        );
    }

    #[test]
    fn test_yoda_condition_snapshots() {
        assert_snapshot!(
            snapshot_lint(
                &[
                    "1 == total",
                    "if (10L < quantity) discount <- 0.1",
                    "1 == 2",
                    "expect_equal(42, calculate_total(items))",
                    "testthat::expect_identical(\"ready\", get_status(job))",
                    "expect_setequal(3L, unique(values))",
                    "expect_equal(1, 1)",
                    "expect_identical(1, 2)",
                    "expect_setequal('a', TRUE)",
                    "expect_equal(expected = total, object = 42)",
                    "expect_equal(7.5, calculate_mean(values), tolerance = 0.1)",
                    "expect_equal(2, # expected\nx)",
                    "test_that(\"你好\", {\n  expect_equal(\n    \"你好\",\n    foo(x)\n  )\n})",
                    // Independent calls inside a pipe must still be checked.
                    "x |> foo(expect_equal(2, y))",
                ]
                .join("\n")
            ),
            @r#"
        warning: yoda_condition
         --> <test>:1:1
          |
        1 | 1 == total
          | ---------- The actual result should come before the expected literal.
          |
          = help: Use `total == 1` instead.
        warning: yoda_condition
         --> <test>:2:5
          |
        2 | if (10L < quantity) discount <- 0.1
          |     -------------- The actual result should come before the expected literal.
          |
          = help: Use `quantity > 10L` instead.
        warning: yoda_condition
         --> <test>:3:1
          |
        3 | 1 == 2
          | ------ Comparing two literals does not test an actual result.
          |
          = help: Compare an actual result with the expected literal instead.
        warning: yoda_condition
         --> <test>:4:1
          |
        4 | expect_equal(42, calculate_total(items))
          | ---------------------------------------- The actual result should be supplied as `object`, and the expected literal as `expected`.
          |
          = help: Use `expect_equal(calculate_total(items), 42)` instead.
        warning: yoda_condition
         --> <test>:5:1
          |
        5 | testthat::expect_identical("ready", get_status(job))
          | ---------------------------------------------------- The actual result should be supplied as `object`, and the expected literal as `expected`.
          |
          = help: Use `testthat::expect_identical(get_status(job), "ready")` instead.
        warning: yoda_condition
         --> <test>:6:1
          |
        6 | expect_setequal(3L, unique(values))
          | ----------------------------------- The actual result should be supplied as `object`, and the expected literal as `expected`.
          |
          = help: Use `expect_setequal(unique(values), 3L)` instead.
        warning: yoda_condition
         --> <test>:7:1
          |
        7 | expect_equal(1, 1)
          | ------------------ Comparing two literals does not test an actual result.
          |
          = help: Compare an actual result with the expected literal instead.
        warning: yoda_condition
         --> <test>:8:1
          |
        8 | expect_identical(1, 2)
          | ---------------------- Comparing two literals does not test an actual result.
          |
          = help: Compare an actual result with the expected literal instead.
        warning: yoda_condition
         --> <test>:9:1
          |
        9 | expect_setequal('a', TRUE)
          | -------------------------- Comparing two literals does not test an actual result.
          |
          = help: Compare an actual result with the expected literal instead.
        warning: yoda_condition
          --> <test>:10:1
           |
        10 | expect_equal(expected = total, object = 42)
           | ------------------------------------------- The actual result should be supplied as `object`, and the expected literal as `expected`.
           |
           = help: Use `expect_equal(total, 42)` instead.
        warning: yoda_condition
          --> <test>:11:1
           |
        11 | expect_equal(7.5, calculate_mean(values), tolerance = 0.1)
           | ---------------------------------------------------------- The actual result should be supplied as `object`, and the expected literal as `expected`.
           |
           = help: Use `expect_equal(calculate_mean(values), 7.5)` instead.
        warning: yoda_condition
          --> <test>:12:1
           |
        12 | / expect_equal(2, # expected
        13 | | x)
           | |__- The actual result should be supplied as `object`, and the expected literal as `expected`.
           |
           = help: Use `expect_equal(x, 2)` instead.
        warning: yoda_condition
          --> <test>:15:3
           |
        15 | /   expect_equal(
        16 | |     "你好",
        17 | |     foo(x)
        18 | |   )
           | |___- The actual result should be supplied as `object`, and the expected literal as `expected`.
           |
           = help: Use `expect_equal(foo(x), "你好")` instead.
        warning: yoda_condition
          --> <test>:20:10
           |
        20 | x |> foo(expect_equal(2, y))
           |          ------------------ The actual result should be supplied as `object`, and the expected literal as `expected`.
           |
           = help: Use `expect_equal(y, 2)` instead.
        Found 14 errors.
        "#
        );

        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "1 == total\n'a' != name\n10L < quantity\n2 <= length(x)\n3 > value\n4 >= value",
                    "if (1 == x) foo()\nwhile (0 < remaining) remaining <- remaining - 1",
                    "dplyr::filter(data, 'ready' == status)",
                    "(1) + 1 == x + 1\n1 == -x\n1 == x * 2\n1 == (x > y)",
                    "1 == !x\n1 == x + !y\n1 == -!x\n1 == if (flag) x else y\n1 == function(x) x",
                    "# leading comment\n'你好'  !=\n  get_name(x) # trailing comment",
                    "expect_equal(TRUE, 1 == total)",
                    "expect_equal(42, calculate_total(items))",
                    "testthat::expect_identical('ready', get_status(job))",
                    "testthat:::expect_setequal(1L, x)",
                    "expect_equal(-1, x)",
                    "expect_equal((2 + 1i), x)",
                    "expect_equal(r\"(a\\b)\", x)",
                    "expect_equal(\n  \"你好\",\n  foo(x)\n)",
                    "# leading comment\nexpect_equal(  2 ,  foo(x)  ) # trailing comment",
                    "x %$% expect_equal(2, value)",
                    "x %>% { expect_equal(2, .) }",
                    "expect_equal(2, x |> foo())",
                    "expect_equal(2, x) |> identity()",
                    "expect_equal(2, expect_identical('a', x))",
                ],
                "yoda_condition"
            )
        );

        assert_snapshot!(
            "no_fix_output",
            get_unsafe_fixed_text(
                vec![
                    "1 == 2\n1 < 2",
                    "1 == # actual\nx",
                    // Two literals, named arguments, and extra arguments have no fix.
                    "expect_equal(1, 1)",
                    "expect_identical(1, 2)",
                    "expect_equal(expected = total, object = 42)",
                    "expect_equal(42, expected = total)",
                    "expect_equal(7.5, calculate_mean(values), tolerance = 0.1)",
                    "expect_equal(2, # expected\nx)",
                    "expect_equal(2, foo( # actual\nx))",
                ],
                "yoda_condition"
            )
        );
    }
}
