use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use reqwest::{
    Client,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
};
use tracing::{debug, instrument, warn};

use crate::spec::{Loader, Manifest, Project};
use crate::{
    modrinth::{
        api::{download_file, request_project, request_versions},
        model::{
            Dependency, DependencyType, File as ModrinthFile, PlatformRequirement,
            Project as ModrinthProject, Version,
        },
    },
    spec::{Artefact, LockfileV1, Spec},
};

fn most_recent_version(versions: &[Version]) -> Option<&Version> {
    versions.iter().max_by_key(|v| &v.date_published)
}

fn primary_files(version: &Version) -> Vec<&ModrinthFile> {
    version.files.iter().filter(|f| f.primary).collect()
}

#[instrument(skip(client, dest, artefacts))]
async fn download_primary_files(
    client: &Client,
    project_id: &str,
    project_slug: &str,
    version_id: &str,
    version_number: &str,
    artefacts: &[&ModrinthFile],
    dest: &Path,
    no_download: bool,
) -> Result<Vec<(PathBuf, Artefact)>> {
    let mut lock_data = Vec::new();
    for artefact in artefacts {
        let artefact_info = download_file(client, project_id, project_slug, version_id, version_number, artefact, dest, no_download).await?;
        lock_data.push(artefact_info);
    }

    Ok(lock_data)
}

fn required_dependencies(version: &Version) -> Vec<&Dependency> {
    version
        .dependencies
        .iter()
        .filter(|d| d.dependency_type == DependencyType::Required)
        .collect()
}

#[instrument(skip_all)]
pub async fn process_manifest(client: &Client, spec: &Spec, output: &Path, no_download: bool, strict: bool) -> Result<LockfileV1> {
    // Process datapacks
    let datapack = process_manifest_for_loader(client, spec, Loader::Datapack, output, no_download, strict).await?;

    // Process fabric mods
    let fabric = process_manifest_for_loader(client, spec, Loader::Fabric, output, no_download, strict).await?;

    Ok(LockfileV1 { datapack, fabric })
}

#[instrument]
pub async fn load_manifest(file_path: &Path) -> Result<Manifest> {
    let file_path = file_path
        .canonicalize()
        .with_context(|| format!("{}: canonicalizing the manifest path", file_path.display()))?;
    let input_file = File::open(&file_path).await.with_context(|| {
        format!(
            "{}: opening the intput file for reading",
            file_path.display()
        )
    })?;
    let mut input_buf = BufReader::new(input_file);
    let mut input_data = Vec::new();
    input_buf
        .read_to_end(&mut input_data)
        .await
        .with_context(|| format!("{}: reading the input file", file_path.display()))?;
    let manifest: Manifest = toml::from_slice(&input_data)
        .with_context(|| format!("{}: parsing the input file as TOML", file_path.display()))?;

    Ok(manifest)
}

#[instrument(skip(manifest))]
pub async fn load_lockfile(manifest: &Manifest, file_path: &Path) -> Result<LockfileV1> {
    let lockfile = if file_path.is_file() {
        let file_path = file_path.canonicalize().with_context(|| {
            format!("{}: canonicalizing the lockfile path", file_path.display())
        })?;

        let lockfile_file = File::open(&file_path).await.with_context(|| {
            format!("{}: opening the lockfile for reading", file_path.display())
        })?;
        let mut lockfile_buf = BufReader::new(lockfile_file);
        let mut lockfile_data = Vec::new();
        lockfile_buf
            .read_to_end(&mut lockfile_data)
            .await
            .with_context(|| format!("{}: reading the lockfile", file_path.display()))?;

        toml::from_slice(&lockfile_data).unwrap_or_else(|e| {
            warn!("returning a default lockfile due to an error deserializing the lockfile: {e}");
            LockfileV1::default()
        })
    } else {
        LockfileV1::default()
    };

    if !(lockfile_is_up_to_date(manifest, &lockfile, Loader::Datapack) && lockfile_is_up_to_date(manifest, &lockfile, Loader::Fabric)) {
        debug!("stale lockfile, discarding its entire contents");
        return Ok(LockfileV1::default());
    }

    Ok(lockfile)
}

fn lockfile_is_up_to_date(manifest: &Manifest, lockfile: &LockfileV1, loader: Loader) -> bool {
    let loader_lockfile = lockfile.get(loader);
    let lockfile_project: HashSet<&str> = loader_lockfile.iter().flat_map(|a| [a.project_id.as_str(), a.project_slug.as_str()].into_iter()).collect();
    let lockfile_version: HashSet<&str> = loader_lockfile.iter().flat_map(|a| [a.version_id.as_str(), a.version_number.as_str()].into_iter()).collect();

    // The lockfile is up-to-date if all entries in the manifest are represented in the
    // lockfile. Meaning, that each manifest project _and_ its version must be found in the
    // lockfile. Wildcard versions (i.e. `None`-valued manifest versions) are treated as a
    // found within the lockfile.
    manifest.get(loader)
        .all(|(n, p)| lockfile_project.contains(n.as_str()) && { if let Some(v) = &p.version {lockfile_version.contains(v.as_str())} else {true}})
}


#[instrument(skip(lockfile))]
pub async fn save_lockfile(lockfile: &LockfileV1, file_path: &Path) -> Result<()> {
    debug!("saving the lockfile");

    let lockfile_data = toml::to_string(lockfile)
        .with_context(|| format!("{lockfile:?}: serializing the lockfile"))?;

    let lockfile_file = File::create(file_path)
        .await
        .with_context(|| format!("{}: opening the lockfile for writing", file_path.display()))?;

    let mut lockfile_buf = BufWriter::new(lockfile_file);
    lockfile_buf
        .write_all(lockfile_data.as_bytes())
        .await
        .with_context(|| format!("{}: writing the lockfile", file_path.display()))?;

    lockfile_buf.flush().await.with_context(|| {
        format!(
            "{}: flushing the lockfile write buffer",
            file_path.display()
        )
    })?;

    Ok(())
}

#[instrument(skip(api_token))]
pub fn http_client(user_agent: &str, api_token: Option<&str>) -> Result<Client> {
    let client = Client::builder()
        .user_agent(user_agent)
        .default_headers({
            let mut headers = HeaderMap::new();
            if let Some(token) = api_token {
                headers.insert(AUTHORIZATION, {
                    let mut bearer = HeaderValue::from_str(&format!("Bearer {token}"))
                        .context("encoding the bearer API token")?;
                    bearer.set_sensitive(true);
                    bearer
                });
            }
            headers
        })
        .build()
        .context("building the HTTP client")?;

    Ok(client)
}

#[instrument(skip_all)]
async fn process_manifest_for_loader(
    client: &Client,
    spec: &Spec,
    loader: Loader,
    output: &Path,
    no_download: bool,
    strict: bool,
) -> Result<Vec<Artefact>> {
    let loader_str = loader.to_string();
    let output = output.join(&loader_str);

    if !output.is_dir() {
        std::fs::create_dir_all(&output).context("creating the output directory")?;
    }

    let versions = collect_versions(client, spec, loader, strict).await?;

    let lock_data = download_artefacts(client, versions.iter(), &output, no_download).await?;

    Ok(lock_data)
}

#[instrument(skip_all)]
async fn download_artefacts(
    client: &Client,
    versions: impl Iterator<Item = &(ModrinthProject, Version)>,
    dest: &Path,
    no_download: bool,
) -> Result<Vec<Artefact>> {
    let mut lock_data_all = Vec::new();
    for (project, version) in versions {
        let primary_artefacts = primary_files(version);
        let mut lock_data = download_primary_files(
            client,
            &version.project_id,
            &project.slug,
            &version.id,
            &version.version_number,
            &primary_artefacts,
            dest,
            no_download,
        )
        .await?;

        lock_data_all.append(&mut lock_data);
    }

    Ok(lock_data_all
        .into_iter()
        .map(|(_, artefact)| artefact)
        .collect())
}

fn validate_server_compatibility(project: &ModrinthProject) -> Result<()> {
    use PlatformRequirement::*;
    match (project.server_side, project.client_side) {
        // Failure
        (Unsupported, _) => {
            return Err(anyhow!(
                "project {}/{} does not support server-side installs",
                project.id,
                project.slug,
            ));
        }

        // Warning
        (_, Required) => {
            warn!(
                "project {}/{} requires a client-side install",
                project.id, project.slug
            );
        }
        (Unknown, _) | (Required, Unknown) | (Optional, Unknown) => warn!(
            "project {}/{} lists either server- or client-side installation requirements as unknown",
            project.id, project.slug,
        ),

        // Success
        _ => (),
    }

    Ok(())
}

#[instrument(skip_all)]
async fn collect_versions(client: &Client, spec: &Spec, loader: Loader, strict: bool) -> Result<HashSet<(ModrinthProject, Version)>> {
    let mut bfs_queue: VecDeque<(ModrinthProject, Version)> = VecDeque::new();
    for (project_name, project_spec) in spec.manifest.get(loader) {
        if let (project, Some(version)) =
            collect_version(client, spec, loader, project_name, project_spec, strict).await?
        {
            bfs_queue.push_back((project, version));
        }
    }

    let mut versions = HashSet::new();
    while let Some((project, version)) = bfs_queue.pop_front() {
        let dependencies = required_dependencies(&version);

        for dep in dependencies {
            let dep_spec = if let Some(vi) = &dep.version_id {
                Project {
                    version: Some(vi.clone()),
                }
            } else {
                Project::default()
            };

            let (dep_project, dep_version) = if let Some(pi) = &dep.project_id {
                match collect_version(client, spec, loader, pi, &dep_spec, strict).await {
                    Ok((project, Some(version))) => (project, version),
                    Ok((_, None)) => continue,
                    Err(e) => return Err(e),
                }
            } else {
                warn!("missing project id");
                continue;
            };

            bfs_queue.push_back((dep_project, dep_version));
        }

        versions.insert((project, version));
    }

    Ok(versions)
}

#[instrument(skip(client, spec))]
async fn collect_version(
    client: &Client,
    spec: &Spec,
    loader: Loader,
    project_name: &str,
    project_spec: &Project,
    strict: bool,
) -> Result<(ModrinthProject, Option<Version>)> {
    let project = request_project(client, &spec.modrinth_api_url, project_name).await?;

    // Verify that the project is not in the denylist
    if spec.denylist.contains(&project.id) || spec.denylist.contains(&project.slug) {
        return Ok((project, None));
    }

    // Verify that the project actually supports server-side installs
    if spec.server_only {
        validate_server_compatibility(&project)?;
    }

    let mut versions = request_versions(
        client,
        &spec.modrinth_api_url,
        &project.id,
        loader,
        &spec.minecraft_version,
    )
    .await?;

    // Retain only versions that match the spec and the lockfile
    let lockfile = spec.lockfile.index(loader);
    versions.retain(|v| {
        // Keep versions whose version number or version id match the corresponding project spec
        // from the manifest (this is an Option type, so iter() will iterate over only one element)
        project_spec.version.iter().all(|pv| pv == &v.version_number || pv == &v.id) &&

        // Keep versions whose version id and project id match those found in the lockfile
        lockfile.contains((&v.project_id, &v.id))
    });

    let Some(version) = most_recent_version(&versions) else {
        if strict {
            return Err(anyhow!("{project_name}: could not find a compatible version"));
        } else {
            warn!("could not find a compatible version");
            return Ok((project, None));
        }
    };

    Ok((project, Some(version.clone())))
}
