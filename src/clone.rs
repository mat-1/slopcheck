use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use eyre::bail;
use url::Url;

use crate::indicators::files::LLM_PATHS;

pub fn clone_repo(url: &Url) -> eyre::Result<PathBuf> {
    let mut url = url.clone();
    let domain = url.domain().unwrap();
    let path = url.path();
    let cache_key = format!("{domain}{path}");
    assert!(!cache_key.contains(".."));
    assert!(domain.chars().next().unwrap().is_alphabetic());

    if domain == "hg.sr.ht" {
        bail!("Tried to clone sourcehut mercurial url: {url}");
    }

    let url_split = url
        .path()
        .split('/')
        // .map(|p| p.to_owned())
        .collect::<Box<[_]>>();
    if url_split.get(3) == Some(&"tree") {
        // fix links like `https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-encoder`
        url.set_path(
            &url_split
                .into_iter()
                .take(3)
                .collect::<Box<[_]>>()
                .join("/"),
        );
    }

    let path = crate::cache_dir("clones", &cache_key);

    let dot_git_path = path.join(".git");
    if dot_git_path.exists() {
        // println!("Using repo {url}");

        let metadata = dot_git_path.metadata().expect("file should exist");
        if let Ok(last_modified) = metadata.modified() {
            let time_since_modified = last_modified.elapsed().unwrap_or_default();
            if time_since_modified > Duration::from_hours(24) {
                let res = Command::new("git")
                    .current_dir(&path)
                    .arg("pull")
                    .status()?;
                if !res.success() {
                    bail!("failed to pull at {path:?}");
                }
            }
        }

        // do checkout every time to make sure new items in LLM_PROMPT_FILES are handled
        // instantly
        checkout_required_files(&path)?;

        return Ok(path);
    }
    // println!("Cloning repo {url}");

    let res = Command::new("git")
        .arg("clone")
        // speed up clones by only downloading metadata
        .arg("--filter=blob:none")
        .arg("--no-checkout")
        .arg(url.to_string())
        .arg(&path)
        .status()?;
    if !res.success() {
        bail!("failed to clone {url}");
    }

    checkout_required_files(&path)?;

    Ok(path)
}

fn checkout_required_files(path: &Path) -> eyre::Result<()> {
    // we still need to look for known LLM files and in the .gitignore, though, so
    // do a sparse checkout for those. also see https://askubuntu.com/a/1074185
    let res = Command::new("git")
        .current_dir(path)
        .arg("sparse-checkout")
        .arg("set")
        .arg("--no-cone")
        .args(LLM_PATHS.iter().map(|p| format!("**/{p}")))
        .arg("**/.gitignore")
        .status()?;
    if !res.success() {
        bail!("failed to do sparse-checkout for {path:?}");
    }

    let res = Command::new("git")
        .current_dir(path)
        .arg("checkout")
        .status()?;
    if !res.success() {
        bail!("failed to do checkout for {path:?}");
    }

    Ok(())
}
