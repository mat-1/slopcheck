use std::path::Path;

use url::Url;

mod cargo;
pub mod npm;

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
pub struct DependencySource {
    pub repo: Url,
}

pub fn get_dep_sources(path: &Path) -> eyre::Result<Box<[DependencySource]>> {
    let mut sources = Vec::new();

    if path.join("Cargo.toml").exists() {
        match cargo::get_dep_sources(path) {
            Ok(d) => sources.extend(d),
            Err(err) => eprintln!("couldn't get dependencies with cargo: {err}"),
        }
    }
    if path.join("package.json").exists() {
        match npm::get_dep_sources(path) {
            Ok(d) => sources.extend(d),
            Err(err) => eprintln!("couldn't get dependencies with npm: {err}"),
        }
    }

    for source in &mut sources {
        // silly way to fix the scheme not being https
        if source.repo.scheme() == "https" {
            continue;
        }
        let repo = source.repo.to_string();
        let (_, repo) = repo.split_once(':').unwrap();
        let repo = format!("https:{repo}");
        let mut repo = Url::parse(&repo)?;
        repo.set_port(None).unwrap();
        source.repo = repo;
    }
    sources.sort();

    Ok(sources.into())
}
