use crate::config::Config;
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Eq, Hash, PartialEq)]
enum Vcs {
    Git,
    Svn,
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct Repository {
    root: String,
    vcs: Vcs,
}

/// Prefer Git, falling back to an SVN working copy when its client is available.
fn discover_repo(path: &str) -> Option<Repository> {
    let dir = match Path::new(path).parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };

    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        && output.status.success()
    {
        return Some(Repository {
            root: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            vcs: Vcs::Git,
        });
    }

    // Start at the working-copy root so files in new, unversioned directories
    // are covered too: `svn info` on those directories would fail.
    let dir = std::fs::canonicalize(dir).ok()?;
    let root = dir.ancestors().find(|dir| dir.join(".svn").is_dir())?;
    let output = Command::new("svn")
        .args(["info", "--show-item", "wc-root", "--non-interactive", "."])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if root.is_empty() {
        return None;
    }
    Some(Repository { root, vcs: Vcs::Svn })
}

/// Get the list of dirty (modified, untracked, staged) files in a repo.
fn dirty_files(repo: &Repository) -> Result<Vec<String>> {
    let (program, args): (&str, &[&str]) = match repo.vcs {
        Vcs::Git => ("git", &["status", "--porcelain"]),
        Vcs::Svn => ("svn", &["status", "--non-interactive", "."]),
    };
    let output = Command::new(program)
        .args(args)
        .current_dir(&repo.root)
        .output()
        .with_context(|| format!("Failed to run `{program} status` in {}", repo.root))?;
    if !output.status.success() {
        bail!(
            "Failed to check version control status in {}: {}",
            repo.root,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if repo.vcs == Vcs::Svn {
        return Ok(svn_dirty_files(&stdout));
    }

    let mut files = Vec::new();
    for line in stdout.lines() {
        // porcelain format: "XY filename" where XY is the two-char status
        if let Some(name) = line.get(3..) {
            files.push(name.to_string());
        }
    }
    Ok(files)
}

fn svn_dirty_files(status: &str) -> Vec<String> {
    status
        .lines()
        .filter(|line| {
            let columns = line.as_bytes();
            // SVN has seven fixed status columns, then a space and the path.
            // Locks, switched paths and externals are not themselves changes.
            matches!(
                columns.first(),
                Some(b'A' | b'C' | b'D' | b'M' | b'R' | b'?' | b'!' | b'~')
            ) || matches!(columns.get(1), Some(b'M' | b'C'))
                || columns.get(6) == Some(&b'C')
        })
        .filter_map(|line| line.get(8..).map(str::to_string))
        .collect()
}

/// Check version control status once for multiple paths.
///
/// The ideal case would be that we know that all paths are either not tracked
/// by VCS or part of the same repo. However, it is completely possible that
/// Jarl is called from a directory where subdirs are different R projects, some
/// not covered by VCS, some covered by VCS but dirty, and some clean.
///
/// Therefore, we cannot just take the first path, check if it's covered by VCS
/// and then get the statuses of all our paths in this repo. We have to loop
/// through paths. This doesn't necessarily result in a big perf hit: what takes
/// time is to get the statuses of the paths, so we limit the calls to statuses
/// by grouping files per repo first. Then, we go through the repos to get the
/// statuses (only once per repo).
pub fn check_version_control(paths: &[String], config: &Config) -> Result<()> {
    if config.allow_no_vcs {
        return Ok(());
    }

    // Group paths by their repository root
    let mut repo_to_paths: HashMap<Repository, Vec<String>> = HashMap::new();
    let mut paths_without_repo: Vec<String> = Vec::new();

    for path in paths {
        match discover_repo(path) {
            Some(repo_root) => {
                repo_to_paths
                    .entry(repo_root)
                    .or_default()
                    .push(path.clone());
            }
            None => {
                paths_without_repo.push(path.clone());
            }
        }
    }

    // Check if any paths are not in a repo
    if !paths_without_repo.is_empty() {
        bail!(
            "`jarl check --fix` can potentially perform destructive changes but no \
            Version Control System (e.g. Git) was found on this project, so no fixes \
            were applied.\n\
            Add `--allow-no-vcs` to the call to apply the fixes."
        )
    }

    if config.allow_dirty {
        return Ok(());
    }

    // Check each repository once
    let mut all_dirty_files = Vec::new();

    for repo_root in repo_to_paths.keys() {
        all_dirty_files.extend(dirty_files(repo_root)?);
    }

    if !all_dirty_files.is_empty() {
        let mut files_list = String::new();
        for file in &all_dirty_files {
            files_list.push_str("  * ");
            files_list.push_str(file);
            files_list.push_str(" (dirty)\n");
        }

        bail!(
            "`jarl check --fix` can potentially perform destructive changes but the working \
            directory of this project has uncommitted changes, so no fixes were applied.\n\
            To apply the fixes, either add `--allow-dirty` to the call, or commit the changes \
            to these files:\n\
             \n\
             {}\n\
             ",
            files_list
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::svn_dirty_files;

    #[test]
    fn svn_status_changes() {
        assert_eq!(
            svn_dirty_files(concat!(
                "A       added.R\n",
                "D       deleted.R\n",
                "M       modified.R\n",
                "R       replaced.R\n",
                "?       untracked.R\n",
                "!       missing.R\n",
                "~       obstructed.R\n",
                "C       conflicted.R\n",
                " M      property.R\n",
                " C      property-conflict.R\n",
                "      C tree-conflict.R\n",
                "      >   local missing, incoming edit upon update\n",
                "M       spaces & @ and \u{4e2d}\u{6587}.R\n",
            )),
            vec![
                "added.R",
                "deleted.R",
                "modified.R",
                "replaced.R",
                "untracked.R",
                "missing.R",
                "obstructed.R",
                "conflicted.R",
                "property.R",
                "property-conflict.R",
                "tree-conflict.R",
                "spaces & @ and \u{4e2d}\u{6587}.R",
            ]
        );
    }

    #[test]
    fn svn_status_metadata_is_not_dirty() {
        assert!(
            svn_dirty_files(concat!(
                "I       ignored.R\n",
                "X       external\n",
                "  L     locked\n",
                "    S   switched\n",
                "     K  locked.R\n",
                "Performing status on external item at 'external':\n",
            ))
            .is_empty()
        );
        assert!(svn_dirty_files("").is_empty());
    }
}
