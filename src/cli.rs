use crate::skin::{
    UpdateStatus, discover_skins, install_remote_skin, remove_skin, update_remote_skins,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Handles CLI arguments. Returns `None` when the GUI should start normally,
/// or `Some(exit_code)` when a CLI subcommand/flag was handled.
pub fn run_from_args<I>(args: I) -> Option<i32>
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();
    let first = args.get(1).map(String::as_str)?;

    match first {
        "-h" | "--help" | "help" => {
            print_help();
            Some(0)
        }
        "-V" | "--version" => {
            println!("arithmetic {VERSION}");
            Some(0)
        }
        "skin" => Some(run_skin_command(&args[2..])),
        other => {
            eprintln!("Unknown command or flag: '{other}'\n");
            print_help();
            Some(1)
        }
    }
}

fn run_skin_command(args: &[String]) -> i32 {
    let Some(sub) = args.first().map(String::as_str) else {
        print_skin_help();
        return 1;
    };

    match sub {
        "-h" | "--help" | "help" => {
            print_skin_help();
            0
        }
        "list" => cmd_list(),
        "add" => cmd_add(&args[1..]),
        "update" => cmd_update(&args[1..]),
        "remove" | "rm" => cmd_remove(&args[1..]),
        other => {
            eprintln!("Unknown skin subcommand: '{other}'\n");
            print_skin_help();
            1
        }
    }
}

fn cmd_list() -> i32 {
    let skins = discover_skins();
    if skins.is_empty() {
        println!("No skins installed.");
        return 0;
    }

    for skin in skins {
        let origin = match &skin.source {
            Some(src) if src.pinned => {
                format!("github:{} @ {} (pinned)", src.repo_slug(), src.ref_name)
            }
            Some(src) => format!("github:{} @ {}", src.repo_slug(), src.short_commit()),
            None => format!("local ({})", skin.directory.display()),
        };
        println!(
            "{:<28} {:<24} v{:<10} {}",
            skin.manifest.id, skin.manifest.name, skin.manifest.version, origin
        );
    }
    0
}

fn cmd_add(args: &[String]) -> i32 {
    let mut repo: Option<&str> = None;
    let mut ref_name: Option<&str> = None;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--ref" => {
                i += 1;
                let Some(val) = args.get(i) else {
                    eprintln!("--ref requires a tag, branch, or commit");
                    return 1;
                };
                ref_name = Some(val.as_str());
            }
            "-h" | "--help" => {
                println!("Usage: arithmetic skin add <owner/repo|github-url> [--ref <tag|branch>]");
                return 0;
            }
            flag if flag.starts_with('-') => {
                eprintln!("Unknown flag: '{flag}'");
                return 1;
            }
            val if repo.is_none() => repo = Some(val),
            extra => {
                eprintln!("Unexpected argument: '{extra}'");
                return 1;
            }
        }
        i += 1;
    }

    let Some(repo) = repo else {
        eprintln!("Usage: arithmetic skin add <owner/repo|github-url> [--ref <tag|branch>]");
        return 1;
    };

    match install_remote_skin(repo, ref_name) {
        Ok(res) => {
            let verb = if res.updated { "Updated" } else { "Installed" };
            let short = res.commit.get(..7).unwrap_or(&res.commit);
            println!("{verb} {} ({}) v{} [{short}]", res.skin_id, res.name, res.version);
            0
        }
        Err(err) => {
            eprintln!("Error: {err}");
            1
        }
    }
}

fn cmd_update(args: &[String]) -> i32 {
    let mut target: Option<&str> = None;
    let mut force = false;

    for arg in args {
        match arg.as_str() {
            "--force" | "-f" => force = true,
            "-h" | "--help" => {
                println!("Usage: arithmetic skin update [skin-id|owner/repo] [--force]");
                return 0;
            }
            flag if flag.starts_with('-') => {
                eprintln!("Unknown flag: '{flag}'");
                return 1;
            }
            val if target.is_none() => target = Some(val),
            extra => {
                eprintln!("Unexpected argument: '{extra}'");
                return 1;
            }
        }
    }

    match update_remote_skins(target, force) {
        Ok(statuses) if statuses.is_empty() => {
            println!("No GitHub-installed skins to update.");
            0
        }
        Ok(statuses) => {
            let mut had_error = false;
            for status in statuses {
                match status {
                    UpdateStatus::Updated(res) => {
                        let short = res.commit.get(..7).unwrap_or(&res.commit);
                        println!("Updated {} to v{} [{short}]", res.skin_id, res.version);
                    }
                    UpdateStatus::UpToDate { skin_id, commit } => {
                        let short = commit.get(..7).unwrap_or(&commit);
                        println!("{skin_id}: already up to date [{short}]");
                    }
                    UpdateStatus::Pinned { skin_id, ref_name } => {
                        println!("{skin_id}: pinned at '{ref_name}' (use --force to re-fetch)");
                    }
                    UpdateStatus::Failed { skin_id, error } => {
                        had_error = true;
                        eprintln!("{skin_id}: update failed: {error}");
                    }
                }
            }
            if had_error { 1 } else { 0 }
        }
        Err(err) => {
            eprintln!("Error: {err}");
            1
        }
    }
}

fn cmd_remove(args: &[String]) -> i32 {
    let Some(target) = args.first().map(String::as_str) else {
        eprintln!("Usage: arithmetic skin remove <skin-id|owner/repo>");
        return 1;
    };
    if matches!(target, "-h" | "--help") {
        println!("Usage: arithmetic skin remove <skin-id|owner/repo>");
        return 0;
    }

    match remove_skin(target) {
        Ok(id) => {
            println!("Removed {id}");
            0
        }
        Err(err) => {
            eprintln!("Error: {err}");
            1
        }
    }
}

fn print_help() {
    println!(
        "\
arithmetic {VERSION}

Usage:
  arithmetic                 Launch the calculator GUI
  arithmetic skin <command>  Manage installed skins
  arithmetic --version       Print version
  arithmetic --help          Show this help

Skin commands:
  arithmetic skin list
  arithmetic skin add <owner/repo|github-url> [--ref <tag|branch>]
  arithmetic skin update [skin-id|owner/repo] [--force]
  arithmetic skin remove <skin-id|owner/repo>"
    );
}

fn print_skin_help() {
    println!(
        "\
Usage:
  arithmetic skin list
  arithmetic skin add <owner/repo|github-url> [--ref <tag|branch>]
  arithmetic skin update [skin-id|owner/repo] [--force]
  arithmetic skin remove <skin-id|owner/repo>"
    );
}

#[cfg(test)]
mod tests {
    use super::run_from_args;

    #[test]
    fn no_args_launches_gui() {
        assert_eq!(run_from_args(["arithmetic".to_string()]), None);
    }

    #[test]
    fn help_and_version_exit_zero() {
        assert_eq!(
            run_from_args(["arithmetic".into(), "--help".into()]),
            Some(0)
        );
        assert_eq!(
            run_from_args(["arithmetic".into(), "--version".into()]),
            Some(0)
        );
    }

    #[test]
    fn unknown_command_exits_one() {
        assert_eq!(
            run_from_args(["arithmetic".into(), "bogus".into()]),
            Some(1)
        );
    }
}
