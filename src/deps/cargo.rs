use std::{collections::HashSet, path::Path, process::Command};

use eyre::bail;
use url::Url;

use crate::deps::DependencySource;

pub fn get_dep_sources(path: &Path) -> eyre::Result<Box<[DependencySource]>> {
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

    Ok(dep_urls
        .into_iter()
        .map(|repo| DependencySource { repo })
        .collect())
}
