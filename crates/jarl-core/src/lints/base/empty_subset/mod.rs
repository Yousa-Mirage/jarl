pub(crate) mod empty_subset;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "empty_subset", None)
    }

    #[test]
    fn test_no_lint_empty_subset() {
        for code in [
            "x[1]",
            "x[[1]]",
            "x[,]",
            "x[, 1]",
            "x[1, ]",
            "x[, drop = FALSE]",
            "x[[,]]",
            "x[i = ]",
            "x[] <- value",
            "x[] <<- value",
            "x[] = value",
            "value -> x[]",
            "value ->> x[]",
            "(x[]) <- value",
            "x[][1] <- value",
            "x[][[1]] <- value",
            "x[]$column <- value",
            "x[]@slot <- value",
            "names(x[]) <- value",
            "names(x = (x[])) <- value",
            "x[ # Preserve the container.\n] <- value",
            "# jarl-ignore empty_subset: custom subsetting method\nx[]",
            "x[",
            "x[[",
            "x <- (1 +\ny <- function(,) 1\nz <- [\n",
        ] {
            expect_no_lint(code, "empty_subset", None);
        }
    }

    #[test]
    fn test_lint_empty_subset() {
        assert_snapshot!(snapshot_lint("x[]"), @"
        warning: empty_subset
         --> <test>:1:2
          |
        1 | x[]
          |  -- Empty `[]` usually selects the entire object.
          |
          = help: Remove the brackets if selecting the entire object is intended.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[[]]"), @"
        warning: empty_subset
         --> <test>:1:2
          |
        1 | x[[]]
          |  ---- Empty `[[]]` normally raises a missing subscript error.
          |
          = help: Supply an index to select an element.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[[]] <- value"), @"
        warning: empty_subset
         --> <test>:1:2
          |
        1 | x[[]] <- value
          |  ---- Empty `[[]]` normally raises a missing subscript error.
          |
          = help: Supply an index to select an element.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("result <- x[]"), @"
        warning: empty_subset
         --> <test>:1:12
          |
        1 | result <- x[]
          |            -- Empty `[]` usually selects the entire object.
          |
          = help: Remove the brackets if selecting the entire object is intended.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[] -> result"), @"
        warning: empty_subset
         --> <test>:1:2
          |
        1 | x[] -> result
          |  -- Empty `[]` usually selects the entire object.
          |
          = help: Remove the brackets if selecting the entire object is intended.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[y[]] <- value"), @"
        warning: empty_subset
         --> <test>:1:4
          |
        1 | x[y[]] <- value
          |    -- Empty `[]` usually selects the entire object.
          |
          = help: Remove the brackets if selecting the entire object is intended.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("f(x, y[]) <- value"), @"
        warning: empty_subset
         --> <test>:1:7
          |
        1 | f(x, y[]) <- value
          |       -- Empty `[]` usually selects the entire object.
          |
          = help: Remove the brackets if selecting the entire object is intended.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("f(x[])"), @"
        warning: empty_subset
         --> <test>:1:4
          |
        1 | f(x[])
          |    -- Empty `[]` usually selects the entire object.
          |
          = help: Remove the brackets if selecting the entire object is intended.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[] + 1"), @"
        warning: empty_subset
         --> <test>:1:2
          |
        1 | x[] + 1
          |  -- Empty `[]` usually selects the entire object.
          |
          = help: Remove the brackets if selecting the entire object is intended.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[[]][]"), @"
        warning: empty_subset
         --> <test>:1:6
          |
        1 | x[[]][]
          |      -- Empty `[]` usually selects the entire object.
          |
          = help: Remove the brackets if selecting the entire object is intended.
        warning: empty_subset
         --> <test>:1:2
          |
        1 | x[[]][]
          |  ---- Empty `[[]]` normally raises a missing subscript error.
          |
          = help: Supply an index to select an element.
        Found 2 errors.
        ");
        assert_snapshot!(snapshot_lint("x[ # Missing index.\n]"), @"
        warning: empty_subset
         --> <test>:1:2
          |
        1 |   x[ # Missing index.
          |  __-
        2 | | ]
          | |_- Empty `[]` usually selects the entire object.
          |
          = help: Remove the brackets if selecting the entire object is intended.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[[ # Missing index.\n]]"), @"
        warning: empty_subset
         --> <test>:1:2
          |
        1 |   x[[ # Missing index.
          |  __-
        2 | | ]]
          | |__- Empty `[[]]` normally raises a missing subscript error.
          |
          = help: Supply an index to select an element.
        Found 1 error.
        ");
    }
}
