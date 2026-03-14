use std::{collections::HashSet, path::Path, process::Command};

use eyre::bail;
use url::Url;

pub fn get_rust_dep_repo_urls(path: &Path) -> eyre::Result<Box<[Url]>> {
    let metadata = Command::new("cargo")
        .current_dir(path)
        .arg("metadata")
        .output()
        .expect("failed to execute process")
        .stdout;
    let metadata = String::from_utf8(metadata).expect("Crate metadata should be UTF-8");
    let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata) else {
        bail!("Failed to parse crate metadata");
    };

    let mut dep_urls = HashSet::new();

    for package in metadata["packages"].as_array().unwrap() {
        let Some(repo_url) = package["repository"].as_str() else {
            println!(
                "dependency is missing repository: {}",
                package["name"].as_str().unwrap()
            );
            continue;
        };

        dep_urls.insert(Url::parse(repo_url).unwrap());
    }

    let mut dep_urls = dep_urls.into_iter().collect::<Box<[_]>>();
    dep_urls.sort();
    Ok(dep_urls)
}
