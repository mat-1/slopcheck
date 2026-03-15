pub mod clone;
pub mod deps;
pub mod indicators;

use std::{
    env,
    fmt::{self, Display},
    path::PathBuf,
    str::FromStr,
};

use git2::Repository;

use crate::{
    clone::clone_repo,
    deps::get_rust_dep_repo_urls,
    indicators::{
        commits::{CommitAuthorsData, check_commit_authors},
        files::{PromptFiles, check_for_prompt_files},
    },
};

fn main() {
    let mut args = env::args();
    if args.len() != 2 {
        println!("usage: slopcheck <path>");
        return;
    }

    // this is required so we don't use the `rust-toolchain.toml` from other
    // crates. it's unset if we're not using rustup.
    if let Some(toolchain) = option_env!("RUSTUP_TOOLCHAIN") {
        // SAFETY: this is only ever run once
        unsafe { env::set_var("RUSTUP_TOOLCHAIN", toolchain) };
    }

    let path = PathBuf::from_str(&args.nth(1).unwrap()).unwrap();
    let path = path.canonicalize().unwrap();

    let mut base_repo_data = None;
    if let Ok(repo) = Repository::open(&path) {
        let commit_authors = check_commit_authors(&repo);
        let commit_authors = match commit_authors {
            Ok(a) => a,
            Err(err) => {
                eprintln!("failed to check commit authors: {err}");
                Default::default()
            }
        };
        let prompt_files = match check_for_prompt_files(&path) {
            Ok(f) => f,
            Err(err) => {
                eprintln!("failed to check for prompt files: {err}");
                Default::default()
            }
        };
        base_repo_data = Some(RepoData {
            identifier: path.to_string_lossy().into(),
            commit_authors,
            prompt_files,
        });
    }

    let mut dep_datas = Vec::new();

    let rust_deps = match get_rust_dep_repo_urls(&path) {
        Ok(d) => d,
        Err(err) => {
            eprintln!("{err}");
            Default::default()
        }
    };
    for dep_url in rust_deps {
        println!("{ITALIC}Checking dependency {BOLD}{dep_url}{RESET}{ITALIC}...{RESET}");
        let dep_path = match clone_repo(&dep_url) {
            Ok(p) => p,
            Err(err) => {
                eprintln!("{err}");
                continue;
            }
        };

        let repo = match Repository::open(&dep_path) {
            Ok(repo) => repo,
            Err(err) => panic!("failed to open: {err}"),
        };
        let prompt_files = match check_for_prompt_files(&dep_path) {
            Ok(f) => f,
            Err(err) => {
                eprintln!("failed to check for prompt files: {err}");
                Default::default()
            }
        };
        let commit_authors = check_commit_authors(&repo).unwrap();
        let repo_data = RepoData {
            identifier: dep_url.to_string().into(),
            commit_authors,
            prompt_files,
        };
        repo_data.maybe_print_summary();
        dep_datas.push(repo_data);
    }

    println!("\n\n{UNDERLINE}Summary:{RESET}\n");
    let mut has_ai = false;
    if let Some(base_repo_data) = base_repo_data
        && base_repo_data.has_ai()
    {
        has_ai = true;
        base_repo_data.maybe_print_summary();
    }
    for dep_data in &dep_datas {
        if dep_data.has_ai() {
            has_ai = true;
            dep_data.maybe_print_summary();
        }
    }

    if !has_ai {
        let or_its_dependencies = if dep_datas.is_empty() {
            ""
        } else {
            " or its dependencies"
        };
        println!(
            "{GREEN}No indicators of LLM use were identified in this project{or_its_dependencies}.{RESET}"
        )
    }

    println!();
}

const RESET: &str = "\x1b[m";
const BOLD: &str = "\x1b[1m";
const ITALIC: &str = "\x1b[3m";
const UNDERLINE: &str = "\x1b[4m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const GRAY: &str = "\x1b[90m";

pub struct RepoData {
    /// Either a URL or a path.
    pub identifier: Box<str>,
    pub commit_authors: CommitAuthorsData,
    pub prompt_files: PromptFiles,
}
impl RepoData {
    pub fn has_ai(&self) -> bool {
        !self.commit_authors.commits_per_llm.is_empty()
            || !self.prompt_files.in_worktree.is_empty()
            || !self.prompt_files.in_gitignore.is_empty()
    }
    pub fn maybe_print_summary(&self) {
        if !self.has_ai() {
            return;
        }

        let llm_actively_being_used = !self.commit_authors.commits_per_llm_in_past_month.is_empty()
            || !self.prompt_files.in_worktree.is_empty()
            || !self.prompt_files.in_gitignore.is_empty();
        let color = if llm_actively_being_used { RED } else { YELLOW };
        println!(
            "{BOLD}{color}{}{RESET}{color} seems to have AI-generated code:{RESET}",
            self.identifier
        );
        maybe_print_summary_for_prompt_files(&self.prompt_files);
        maybe_print_summary_for_commits(&self.commit_authors);
    }
}

pub fn maybe_print_summary_for_prompt_files(data: &PromptFiles) {
    let (found_where, files) = if !data.in_worktree.is_empty() {
        ("working tree", &*data.in_worktree)
    } else if !data.in_gitignore.is_empty() {
        (".gitignore", &*data.in_gitignore)
    } else {
        return;
    };

    let plural_suffix = if data.in_worktree.len() == 1 { "" } else { "s" };
    println!(
        "  {GRAY}{ITALIC}LLM-related path{plural_suffix} in {found_where}: {}.{RESET}",
        format_list(files, &format!("{GRAY}{ITALIC}"))
    )
}
fn format_list(list: &[&str], default_style: &str) -> String {
    let mut s = String::new();
    for (i, item) in list.iter().enumerate() {
        if i != 0 {
            if list.len() == 2 {
                s.push(' ');
            } else {
                s.push_str(", ");
            }
            if i == list.len() - 1 {
                s.push_str("and ");
            }
        }
        s.push_str(BOLD);
        s.push_str(item);
        s.push_str(RESET);
        s.push_str(default_style);
    }
    s
}

pub fn maybe_print_summary_for_commits(data: &CommitAuthorsData) {
    if data.commits_per_llm.is_empty() {
        return;
    }

    let total_llm_commits = data.commits_per_llm.iter().map(|(_, c)| c).sum::<u64>();
    let total_llm_commits_in_past_month = data
        .commits_per_llm_in_past_month
        .iter()
        .map(|(_, c)| c)
        .sum::<u64>();
    let plural_suffix = if total_llm_commits != 1 { "s" } else { "" };
    print!("{GRAY}{ITALIC}");
    print!(
        "  {BOLD}{total_llm_commits}{RESET}{GRAY}{ITALIC} LLM-authored commit{plural_suffix} ({total_llm_commits_in_past_month} in past month) -- "
    );
    if data.commits_per_llm.len() == 1 {
        let llm_name = data.commits_per_llm[0];
        print!("all by {}.", llm_name.0);
    } else {
        for (i, (llm_name, count)) in data.commits_per_llm.iter().enumerate() {
            if i != 0 {
                print!(", ")
            }
            print!("{count} by {llm_name}");
        }
        print!(".");
    }
    println!("{RESET}");
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct LlmName(&'static str);
impl Display for LlmName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
