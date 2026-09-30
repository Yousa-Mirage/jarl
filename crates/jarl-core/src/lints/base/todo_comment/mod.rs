pub(crate) mod todo_comment;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    #[test]
    fn test_no_lint_todo_comment() {
        for code in [
            "",
            "x <- 1",
            "# ordinary comment",
            "# TO DO: later",
            "# TODOLOGY",
            "# TODOa123",
            "# fixmeLater",
            "# TODO中文",
            "# FIXMEé",
            "# TODOλ",
            "# FIXMEЖ",
            "# remember TODO: later",
            "# text # TODO: later",
            "#' @description TODO: later",
            "# @TODO",
            "# | TODO",
            r##"x <- "# TODO""##,
            r##"x <- r"(# FIXME)""##,
            "x <- \"line one\n# TODO\nline three\"",
            "# jarl-ignore-file todo_comment: tracked elsewhere\n# TODO: later",
            "# jarl-ignore-start todo_comment: tracked elsewhere\n# FIXME\n# jarl-ignore-end todo_comment",
        ] {
            expect_no_lint(code, "todo_comment", None);
        }
    }

    #[test]
    fn test_lint_todo_comment() {
        let code = [
            "# TODO",
            "# FIXME",
            "# tOdO later",
            "# fIxMe\tlater",
            "#TODO123",
            "# TODO123abc",
            "# FIXME９",
            "# TODO: later",
            "# FIXME(issue): later",
            "# TODO_task",
            "# FIXME.md",
            "# TODO🛠",
            "### TODO: later",
            "#\tTODO\u{a0}later",
            "#' FIXME: document this",
            "x <- 1 # TODO: later",
            "function() {\n  # FIXME: later\n  x <- 1\n}",
            "# TODO and FIXME: one diagnostic",
        ]
        .join("\n");

        assert_snapshot!(format_diagnostics(&code, "todo_comment", None));
    }

    #[test]
    fn test_todo_comment_roxygen_examples() -> anyhow::Result<()> {
        use crate::check::check;
        use crate::config::{ArgsConfig, build_config};
        use std::fs;

        let directory = tempfile::tempdir()?;
        fs::write(
            directory.path().join("DESCRIPTION"),
            "Package: fixture\nVersion: 0.0.1\n",
        )?;
        fs::create_dir(directory.path().join("R"))?;
        let file = directory.path().join("R/example.R");
        let code = "\
#' TODO: finish docs
#' @examples
#' # jarl-ignore unused_object: illustrative code
#' x <- 1 # TODO: example code
#' # FIXME: example comment
f <- function() NULL
";
        fs::write(&file, code)?;
        let args = ArgsConfig {
            files: vec![file.clone()],
            fix: false,
            unsafe_fixes: false,
            fix_only: false,
            select: "todo_comment".to_string(),
            extend_select: String::new(),
            ignore: String::new(),
            min_r_version: None,
            allow_dirty: false,
            allow_no_vcs: true,
            assignment: None,
        };
        let config = build_config(&args, None, vec![file])?;
        let diagnostics = check(config)
            .into_iter()
            .next()
            .expect("fixture file was checked")
            .1?;
        let comments: Vec<_> = diagnostics
            .iter()
            .map(|diagnostic| {
                &code[usize::from(diagnostic.range.start())..usize::from(diagnostic.range.end())]
            })
            .collect();

        // Examples must not reinterpret markers in the middle of roxygen comments.
        assert_eq!(comments, ["#' TODO: finish docs"]);
        Ok(())
    }
}
