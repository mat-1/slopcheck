use std::{
    collections::{HashSet, VecDeque},
    fs,
    path::Path,
    sync::LazyLock,
    time::Duration,
};

use ureq::Agent;
use url::Url;

use crate::deps::DependencySource;

pub fn get_dep_sources(path: &Path) -> eyre::Result<Box<[DependencySource]>> {
    let package_json = fs::read_to_string(path.join("package.json"))?;
    let deps = extract_dependencies_from_package_json(&package_json)?;

    let mut sources = Vec::new();

    let mut checked = HashSet::<String>::new();

    let mut queue = deps.into_iter().collect::<VecDeque<NpmDependency>>();

    while let Some(dep) = queue.pop_front() {
        println!("Checking dependency {}", dep.name);
        let data = fetch_npm_metadata(&dep.name)?;
        let data = serde_json::from_str::<serde_json::Value>(&data)?;

        let Some(repository) = data["repository"]["url"].as_str() else {
            continue;
        };

        let mut repository = repository.split("+").last().unwrap();
        if let Some(r) = repository.strip_suffix(".git") {
            repository = r;
        }
        let mut repository = repository.to_owned();
        if let Some(r) = repository
            .strip_prefix("git@")
            .or_else(|| repository.strip_prefix("ssh://git@"))
        {
            repository = format!("https://{}", r.replacen(':', "://", 1));
        }
        println!("  Repository: {repository}");

        if checked.contains(&repository) {
            continue;
        }
        checked.insert(repository.clone());

        sources.push(DependencySource {
            repo: Url::parse(&repository)?,
        });

        // don't bother actually parsing versions, just use the latest one lol
        let Some(version) = data["dist-tags"]["latest"].as_str() else {
            panic!("no latest version on {}", dep.name);
        };
        let version_data = &data["versions"][version];

        let version_deps_json = &version_data["dependencies"];
        // don't include dev dependencies from dependencies since the user
        // wouldn't be pulling them in anyways and they add too much bloat

        if let Some(version_deps_json) = version_deps_json.as_object() {
            for (name, version) in version_deps_json {
                let Some(version) = version.as_str() else {
                    continue;
                };
                queue.push_back(NpmDependency {
                    name: name.to_owned(),
                    version: version.to_owned(),
                });
            }
        }
    }

    println!("sources: {sources:?}");

    Ok(sources.into())
}

static AGENT: LazyLock<Agent> = LazyLock::new(|| {
    Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into()
});

pub fn fetch_npm_metadata(package_name: &str) -> eyre::Result<String> {
    let cache_dir = crate::cache_dir("npm", &format!("{package_name}.json"));
    if let Ok(metadata) = cache_dir.metadata()
        && let Ok(last_modified) = metadata.modified()
    {
        let time_since_modified = last_modified.elapsed().unwrap_or_default();
        if time_since_modified < Duration::from_hours(24) {
            return Ok(fs::read_to_string(cache_dir)?);
        }
    }

    println!("  Fetching metadata for {package_name} from NPM");
    let npm_metadata_url = format!("https://registry.npmjs.org/{package_name}");
    let metadata = AGENT
        .get(npm_metadata_url)
        .call()?
        .into_body()
        .read_to_string()?;
    fs::create_dir_all(cache_dir.parent().unwrap())?;
    fs::write(cache_dir, &metadata)?;
    Ok(metadata)
}

#[derive(Debug)]
pub struct NpmDependency {
    pub name: String,
    pub version: String,
}

fn extract_dependencies_from_package_json(contents: &str) -> eyre::Result<Box<[NpmDependency]>> {
    let package_json = serde_json::from_str::<serde_json::Value>(contents)?;
    let dependencies = package_json.get("dependencies");
    let dev_dependencies = package_json.get("devDependencies");

    let mut all_deps = Vec::new();

    if let Some(dependencies) = dependencies.and_then(|d| d.as_object()) {
        for (k, v) in dependencies {
            let Some(v) = v.as_str() else { continue };
            all_deps.push(NpmDependency {
                name: k.to_owned(),
                version: v.to_owned(),
            })
        }
    }
    if let Some(dev_dependencies) = dev_dependencies.and_then(|d| d.as_object()) {
        for (k, v) in dev_dependencies {
            let Some(v) = v.as_str() else { continue };
            all_deps.push(NpmDependency {
                name: k.to_owned(),
                version: v.to_owned(),
            })
        }
    }

    Ok(all_deps.into())
}
