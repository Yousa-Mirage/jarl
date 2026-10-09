use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};
use url::Url;

use crate::helpers::{CliTest, CommandExt, create_commit, git_init};

const CODE: &str = "any(is.na(x))";

fn svn(dir: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("svn")
        .args(args)
        .arg("--non-interactive")
        .current_dir(dir)
        .output()?;
    if !output.status.success() {
        bail!("svn {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

fn working_copy() -> Result<Option<CliTest>> {
    // SVN is optional for Jarl users and developers; Linux CI installs it.
    for program in ["svn", "svnadmin"] {
        if Command::new(program).arg("--version").output().is_err() {
            eprintln!("Skipping SVN integration test: {program} is not installed");
            return Ok(None);
        }
    }
    let case = CliTest::new()?;
    let repository = case.root().join("repository");
    let output = Command::new("svnadmin")
        .arg("create")
        .arg(&repository)
        .output()?;
    if !output.status.success() {
        bail!(
            "svnadmin create: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let url = Url::from_directory_path(&repository)
        .map_err(|()| anyhow::anyhow!("Invalid local SVN repository path"))?;
    svn(case.root(), &["checkout", url.as_str(), "wc"])?;
    case.write_file("wc/R/test.R", CODE)?;
    case.write_file("wc/notes.txt", "initial")?;
    svn(&case.root().join("wc"), &["add", "R", "notes.txt"])?;
    svn(&case.root().join("wc"), &["commit", "-m", "Initial commit"])?;
    svn(&case.root().join("wc"), &["update"])?;
    Ok(Some(case))
}

#[test]
fn clean_svn_working_copy() -> Result<()> {
    let Some(case) = working_copy()? else {
        return Ok(());
    };
    insta::assert_snapshot!(
        case.command().args(["check", "wc/R", "--fix"]).run(),
        @"
    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );
    assert_eq!(case.read_file("wc/R/test.R")?, "anyNA(x)");
    Ok(())
}

#[test]
fn modified_svn_file_blocks_fix() -> Result<()> {
    let Some(case) = working_copy()? else {
        return Ok(());
    };
    case.write_file("wc/notes.txt", "modified")?;
    insta::assert_snapshot!(
        case.command().args(["check", "wc/R/test.R", "--fix"]).run(),
        @"
    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    Error: `jarl check --fix` can potentially perform destructive changes but the working directory of this project has uncommitted changes, so no fixes were applied.
    To apply the fixes, either add `--allow-dirty` to the call, or commit the changes to these files:

      * notes.txt (dirty)
    "
    );
    assert_eq!(case.read_file("wc/R/test.R")?, CODE);
    Ok(())
}

#[test]
fn svn_conflict_blocks_fix() -> Result<()> {
    let Some(case) = working_copy()? else {
        return Ok(());
    };
    let repository = Url::from_directory_path(case.root().join("repository"))
        .map_err(|()| anyhow::anyhow!("Invalid local SVN repository path"))?;
    svn(case.root(), &["checkout", repository.as_str(), "other"])?;
    case.write_file("wc/notes.txt", "local change\n")?;
    case.write_file("other/notes.txt", "incoming change\n")?;
    svn(
        &case.root().join("other"),
        &["commit", "-m", "Incoming change"],
    )?;
    svn(&case.root().join("wc"), &["update"])?;

    let output = case.command().args(["check", "wc", "--fix"]).run();
    assert_eq!(output.status.code(), Some(255));
    assert!(output.stderr.contains("notes.txt (dirty)"));
    assert_eq!(case.read_file("wc/R/test.R")?, CODE);
    Ok(())
}

#[test]
fn multiple_svn_working_copies() -> Result<()> {
    let Some(case) = working_copy()? else {
        return Ok(());
    };
    let repository = Url::from_directory_path(case.root().join("repository"))
        .map_err(|()| anyhow::anyhow!("Invalid local SVN repository path"))?;
    svn(case.root(), &["checkout", repository.as_str(), "other"])?;
    let output = case.command().args(["check", "wc", "other", "--fix"]).run();
    assert!(output.status.success(), "{output}");
    assert_eq!(case.read_file("wc/R/test.R")?, "anyNA(x)");
    assert_eq!(case.read_file("other/R/test.R")?, "anyNA(x)");

    case.write_file("wc/R/test.R", CODE)?;
    let output = case.command().args(["check", "wc", "other", "--fix"]).run();
    assert_eq!(output.status.code(), Some(255));
    assert!(output.stderr.contains("uncommitted changes"));
    assert_eq!(case.read_file("wc/R/test.R")?, CODE);
    Ok(())
}

#[test]
fn git_is_preferred_when_both_are_present() -> Result<()> {
    let Some(case) = working_copy()? else {
        return Ok(());
    };
    let wc = case.root().join("wc");
    git_init(&wc)?;
    let output = Command::new("git")
        .args(["add", "R"])
        .current_dir(&wc)
        .output()?;
    assert!(output.status.success());
    create_commit(&wc.join("notes.txt"), &wc)?;
    let output = case.command().args(["check", "wc", "--fix"]).run();
    assert_eq!(output.status.code(), Some(255));
    // SVN is clean, but Git sees the untracked .svn metadata directory.
    assert!(output.stderr.contains(".svn/ (dirty)"), "{output}");
    assert_eq!(case.read_file("wc/R/test.R")?, CODE);
    Ok(())
}

#[test]
fn svn_property_changes_block_fix() -> Result<()> {
    let Some(case) = working_copy()? else {
        return Ok(());
    };
    svn(
        &case.root().join("wc"),
        &["propset", "test-property", "value", "."],
    )?;
    let output = case.command().args(["check", "wc", "--fix"]).run();
    assert_eq!(output.status.code(), Some(255));
    assert!(output.stderr.contains("uncommitted changes"));
    assert_eq!(case.read_file("wc/R/test.R")?, CODE);
    Ok(())
}

#[test]
fn unversioned_svn_directory_is_covered_but_dirty() -> Result<()> {
    let Some(case) = working_copy()? else {
        return Ok(());
    };
    case.write_file("wc/new/nested/test.R", CODE)?;
    let output = case.command().args(["check", "wc/new", "--fix"]).run();
    assert_eq!(output.status.code(), Some(255));
    assert!(output.stderr.contains("uncommitted changes"));
    assert!(!output.stderr.contains("no Version Control System"));
    assert_eq!(case.read_file("wc/new/nested/test.R")?, CODE);

    let output = case
        .command()
        .args(["check", "wc/new", "--fix", "--allow-dirty"])
        .run();
    assert!(output.status.success(), "{output}");
    assert_eq!(case.read_file("wc/new/nested/test.R")?, "anyNA(x)");
    Ok(())
}

#[test]
fn svn_add_delete_and_missing_items_block_fix() -> Result<()> {
    let Some(case) = working_copy()? else {
        return Ok(());
    };
    let wc = case.root().join("wc");
    case.write_file("wc/added.txt", "added")?;
    svn(&wc, &["add", "added.txt"])?;
    let output = case.command().args(["check", "wc", "--fix"]).run();
    assert_eq!(output.status.code(), Some(255));
    assert!(output.stderr.contains("added.txt (dirty)"));
    assert_eq!(case.read_file("wc/R/test.R")?, CODE);

    svn(&wc, &["commit", "-m", "Add file"])?;
    svn(&wc, &["delete", "added.txt"])?;
    let output = case.command().args(["check", "wc", "--fix"]).run();
    assert_eq!(output.status.code(), Some(255));
    assert!(output.stderr.contains("added.txt (dirty)"));
    assert_eq!(case.read_file("wc/R/test.R")?, CODE);

    svn(&wc, &["commit", "-m", "Delete file"])?;
    std::fs::remove_file(wc.join("notes.txt"))?;
    let output = case.command().args(["check", "wc", "--fix"]).run();
    assert_eq!(output.status.code(), Some(255));
    assert!(output.stderr.contains("notes.txt (dirty)"));
    assert_eq!(case.read_file("wc/R/test.R")?, CODE);
    Ok(())
}

#[test]
fn dirty_svn_does_not_block_lint() -> Result<()> {
    let Some(case) = working_copy()? else {
        return Ok(());
    };
    case.write_file("wc/notes.txt", "modified")?;
    let output = case.command().args(["check", "wc"]).run();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.contains("warning: any_is_na"));
    assert_eq!(case.read_file("wc/R/test.R")?, CODE);
    Ok(())
}

#[test]
fn allow_dirty_and_allow_no_vcs_bypass_svn_changes() -> Result<()> {
    let Some(case) = working_copy()? else {
        return Ok(());
    };
    case.write_file("wc/notes.txt", "modified")?;
    let output = case
        .command()
        .args(["check", "wc", "--fix", "--allow-dirty"])
        .run();
    assert!(output.status.success(), "{output}");
    assert_eq!(case.read_file("wc/R/test.R")?, "anyNA(x)");

    case.write_file("wc/R/test.R", CODE)?;
    let output = case
        .command()
        .args(["check", "wc", "--fix", "--allow-no-vcs"])
        .run();
    assert!(output.status.success(), "{output}");
    assert_eq!(case.read_file("wc/R/test.R")?, "anyNA(x)");
    Ok(())
}

#[test]
fn ignored_svn_items_do_not_make_working_copy_dirty() -> Result<()> {
    let Some(case) = working_copy()? else {
        return Ok(());
    };
    let wc = case.root().join("wc");
    svn(&wc, &["propset", "svn:ignore", "ignored.txt", "."])?;
    svn(&wc, &["commit", "-m", "Set ignore"])?;
    case.write_file("wc/ignored.txt", "ignored")?;
    let output = case.command().args(["check", "wc", "--fix"]).run();
    assert!(output.status.success(), "{output}");
    assert_eq!(case.read_file("wc/R/test.R")?, "anyNA(x)");
    Ok(())
}

#[test]
fn mixed_git_svn_and_uncovered_files() -> Result<()> {
    let Some(case) = working_copy()? else {
        return Ok(());
    };
    case.write_file("git/test.R", CODE)?;
    let git = case.root().join("git");
    git_init(&git)?;
    create_commit(&git.join("test.R"), &git)?;
    let output = case.command().args(["check", "git", "wc", "--fix"]).run();
    assert!(output.status.success(), "{output}");
    assert_eq!(case.read_file("git/test.R")?, "anyNA(x)");
    assert_eq!(case.read_file("wc/R/test.R")?, "anyNA(x)");

    let output = case.command().args(["check", "git", "wc", "--fix"]).run();
    assert_eq!(output.status.code(), Some(255));
    assert!(output.stderr.contains("uncommitted changes"));

    case.write_file("uncovered/test.R", CODE)?;
    let output = case
        .command()
        .args(["check", "wc", "uncovered", "--fix", "--allow-dirty"])
        .run();
    assert_eq!(output.status.code(), Some(255));
    assert!(output.stderr.contains("no Version Control System"));
    assert_eq!(case.read_file("uncovered/test.R")?, CODE);
    Ok(())
}

#[test]
fn missing_svn_client_is_not_recognized_as_vcs() -> Result<()> {
    let Some(case) = working_copy()? else {
        return Ok(());
    };
    let output = case
        .command()
        .env("PATH", "")
        .args(["check", "wc", "--fix", "--allow-dirty"])
        .run();
    assert_eq!(output.status.code(), Some(255));
    assert!(output.stderr.contains("no Version Control System"));
    assert_eq!(case.read_file("wc/R/test.R")?, CODE);

    let output = case
        .command()
        .env("PATH", "")
        .args(["check", "wc", "--fix", "--allow-no-vcs"])
        .run();
    assert!(output.status.success(), "{output}");
    assert_eq!(case.read_file("wc/R/test.R")?, "anyNA(x)");
    Ok(())
}

#[cfg(unix)]
#[test]
fn failed_svn_status_does_not_allow_fix() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let Some(case) = working_copy()? else {
        return Ok(());
    };
    case.write_file(
        "bin/svn",
        "#!/bin/sh\nif [ \"$1\" = info ]; then pwd; else echo 'status failed' >&2; exit 1; fi\n",
    )?;
    let script = case.root().join("bin/svn");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))?;
    let output = case
        .command()
        .env("PATH", case.root().join("bin"))
        .args(["check", "wc", "--fix"])
        .run();
    assert_eq!(output.status.code(), Some(255));
    assert!(
        output
            .stderr
            .contains("Failed to check version control status"),
        "{output}"
    );
    assert!(output.stderr.contains("status failed"));
    assert_eq!(case.read_file("wc/R/test.R")?, CODE);
    Ok(())
}
