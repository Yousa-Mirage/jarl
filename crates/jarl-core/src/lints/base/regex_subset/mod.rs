pub(crate) mod regex_subset;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str, min_r_version: Option<&str>) -> String {
        format_diagnostics(code, "regex_subset", min_r_version)
    }

    #[test]
    fn test_no_lint_regex_subset() {
        expect_no_lint("y[grepl(pattern, x)]", "regex_subset", None);
        expect_no_lint("x[grepl(pattern, foo(x))]", "regex_subset", None);
        expect_no_lint("x[grep(pattern, x), ]", "regex_subset", None);
        expect_no_lint("x[, grep(pattern, x)]", "regex_subset", None);
        expect_no_lint("x[foo = grep(pattern, x)]", "regex_subset", None);
        expect_no_lint("x[index]", "regex_subset", None);
        expect_no_lint("x[grep()]", "regex_subset", None);
        expect_no_lint("x[grep(pattern, x)] <- value", "regex_subset", None);
        expect_no_lint("x[grepl(pattern, x)] = value", "regex_subset", None);
        expect_no_lint("value -> x[grep(pattern, x)]", "regex_subset", None);
        expect_no_lint(
            "x[stringr::str_detect(foo(x), pattern)]",
            "regex_subset",
            None,
        );
        expect_no_lint("y[stringr::str_which(x, pattern)]", "regex_subset", None);
        expect_no_lint("grepv(pattern, x)", "regex_subset", None);
        expect_no_lint("stringr::str_subset(x, pattern)", "regex_subset", None);
    }

    #[test]
    fn test_lint_regex_subset_base_r_4_5() {
        assert_snapshot!(
            snapshot_lint("x[grep(pattern, x)]", Some("4.5")),
            @"
        warning: regex_subset
         --> <test>:1:1
          |
        1 | x[grep(pattern, x)]
          | ------------------- `grepv(pattern, x)` is more direct than `x[grep(pattern, x)]`.
          |
          = help: Use `grepv(pattern, x)` instead.
        Found 1 error.
        "
        );
        assert_eq!(
            check_code(
                "x[grep(pattern, x)] | condition",
                "regex_subset",
                Some("4.5")
            )
            .len(),
            1
        );
        assert_snapshot!(
            snapshot_lint(
                "names(y)[grepl(pattern, names(y), perl = TRUE)]",
                Some("4.5")
            ),
            @"
        warning: regex_subset
         --> <test>:1:1
          |
        1 | names(y)[grepl(pattern, names(y), perl = TRUE)]
          | ----------------------------------------------- `grepv(pattern, names(y), perl = TRUE)` is more direct than `names(y)[grepl(pattern, names(y), perl = TRUE)]`.
          |
          = help: Use `grepv(pattern, names(y), perl = TRUE)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint(
                "names(foo(y))[base::grepl(pattern, names(foo(y)), fixed = TRUE)]",
                Some("4.5")
            ),
            @"
        warning: regex_subset
         --> <test>:1:1
          |
        1 | names(foo(y))[base::grepl(pattern, names(foo(y)), fixed = TRUE)]
          | ---------------------------------------------------------------- `base::grepv(pattern, names(foo(y)), fixed = TRUE)` is more direct than `names(foo(y))[base::grepl(pattern, names(foo(y)), fixed = TRUE)]`.
          |
          = help: Use `base::grepv(pattern, names(foo(y)), fixed = TRUE)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_regex_subset_before_r_4_5() {
        assert_snapshot!(
            snapshot_lint("x[grepl(pattern, x)]", Some("4.4")),
            @"
        warning: regex_subset
         --> <test>:1:1
          |
        1 | x[grepl(pattern, x)]
          | -------------------- `grep(pattern, x, value = TRUE)` is more direct than `x[grepl(pattern, x)]`.
          |
          = help: Use `grep(pattern, x, value = TRUE)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x[grep(pattern, x)]", None),
            @"
        warning: regex_subset
         --> <test>:1:1
          |
        1 | x[grep(pattern, x)]
          | ------------------- `grep(pattern, x, value = TRUE)` is more direct than `x[grep(pattern, x)]`.
          |
          = help: Use `grep(pattern, x, value = TRUE)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_regex_subset_stringr() {
        assert_snapshot!(
            snapshot_lint("x[stringr::str_which(x, pattern)]", None),
            @"
        warning: regex_subset
         --> <test>:1:1
          |
        1 | x[stringr::str_which(x, pattern)]
          | --------------------------------- `stringr::str_subset(x, pattern)` is more direct than `x[stringr::str_which(x, pattern)]`.
          |
          = help: Use `stringr::str_subset(x, pattern)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("names(y)[str_detect(names(y), pattern, negate = TRUE)]", None),
            @"
        warning: regex_subset
         --> <test>:1:1
          |
        1 | names(y)[str_detect(names(y), pattern, negate = TRUE)]
          | ------------------------------------------------------ `str_subset(names(y), pattern, negate = TRUE)` is more direct than `names(y)[str_detect(names(y), pattern, negate = TRUE)]`.
          |
          = help: Use `str_subset(names(y), pattern, negate = TRUE)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_fix_regex_subset() {
        assert_snapshot!(
            "fix_output_before_r_4_5",
            get_unsafe_fixed_text_with_settings(
                vec![
                    "x[grep(pattern, x)]",
                    "x[grepl(pattern, x)]",
                    "x[stringr::str_which(x, pattern)]",
                    "x[stringr::str_detect(x, pattern)]",
                ],
                "regex_subset",
                Some("4.4"),
                None,
            )
        );
        assert_snapshot!(
            "fix_output_r_4_5",
            get_unsafe_fixed_text_with_settings(
                vec![
                    "x[grep(pattern, x)]",
                    "x[grepl(pattern, x)]",
                    "x[grep(pattern, x, value = FALSE, fixed = TRUE)]",
                    "x[grep(pattern, x, FALSE, FALSE, FALSE, TRUE, TRUE)]",
                    "x[grepl(pattern, x, FALSE, FALSE, TRUE, TRUE)]",
                    "x[stringr::str_which(x, pattern)]",
                    "x[stringr::str_detect(x, pattern)]",
                ],
                "regex_subset",
                Some("4.5"),
                None,
            )
        );
    }

    #[test]
    fn test_regex_subset_comments() {
        assert_eq!(
            check_code(
                "names(y # comment\n)[grepl(pattern, names(y))]",
                "regex_subset",
                Some("4.5")
            )
            .len(),
            1
        );

        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text(
                vec![
                    "# leading comment\nx[grep(pattern, x)]",
                    "x[\n  # comment\n  grep(pattern, x)\n]",
                    "x[grep(\n  # comment\n  pattern, x\n)]",
                    "x[grep(pattern, x)] # trailing comment",
                ],
                "regex_subset",
            )
        );
    }
}
