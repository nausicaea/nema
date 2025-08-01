use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use reqwest::{
    Client, Response, Url,
    header::{CONTENT_LENGTH, CONTENT_TYPE},
};
use sha2::{Digest, Sha512};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
};
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;
use tracing::{debug, instrument, warn};

use crate::spec::{Artefact, Loader};

use super::model::{File as ModrinthFile, Project, Version};

pub const MODRINTH_PROD_BASE_URL: &str = "https://api.modrinth.com";
pub const MODRINTH_STAGING_BASE_URL: &str = "https://staging-api.modrinth.com";

#[instrument(skip(client, base_url))]
pub async fn request_project(client: &Client, base_url: &Url, project_id: &str) -> Result<Project> {
    let url = base_url.join(&format!("/v2/project/{project_id}"))?;
    let request = client.get(url).build()?;
    let project: Project = client.execute(request).await?.json().await?;

    Ok(project)
}

#[instrument(skip(client, base_url))]
pub async fn request_version(
    client: &Client,
    base_url: &Url,
    project_id: &str,
    version_id: &str,
) -> Result<Version> {
    let url = base_url.join(&format!("/v2/project/{project_id}/version/{version_id}"))?;
    let request = client.get(url).build()?;
    let version: Version = client.execute(request).await?.json().await?;

    Ok(version)
}

#[instrument(skip(client, base_url))]
pub async fn request_versions(
    client: &Client,
    base_url: &Url,
    project_name: &str,
    loader: Loader,
    minecraft_version: &str,
) -> Result<Vec<Version>> {
    let url = base_url.join(&format!(
        "/v2/project/{project_name}/version?loaders=%5B%22{loader}%22%5D&game_versions=%5B%22{minecraft_version}%22%5D"
    ))?;
    let request = client.get(url).build()?;
    let mut versions: Vec<Version> = client.execute(request).await?.json().await?;

    // Work around a bug where the above REST API endpoint would sometimes return results without
    // the specified loader
    versions.retain(|v| v.loaders.iter().any(|l| loader == l));

    Ok(versions)
}

#[instrument(skip(client, dest, project_id, version_id))]
pub async fn download_file(
    client: &Client,
    project_id: &str,
    project_slug: &str,
    version_id: &str,
    version_number: &str,
    artefact: &ModrinthFile,
    dest: &Path,
    no_download: bool,
) -> Result<(PathBuf, Artefact)> {
    let artefact_path = dest.join(&artefact.filename);
    let artefact_info = Artefact::new(project_id, project_slug, version_id, version_number, artefact)?;

    if no_download {
        debug!("downloads are disabled, so the artefact path will not exist");
        return Ok((artefact_path, artefact_info));
    }

    if !artefact_path.exists() {
        let url = Url::parse(&artefact.url).with_context(|| artefact_info.clone())?;
        let request = client
            .get(url)
            .build()
            .with_context(|| artefact_info.clone())?;
        let response = client
            .execute(request)
            .await
            .with_context(|| artefact_info.clone())?;

        validate_content_type(&response).with_context(|| artefact_info.clone())?;

        validate_content_length(&response).with_context(|| artefact_info.clone())?;

        // Write the body data to the destination
        {
            let artefact_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&artefact_path)
                .await
                .with_context(|| artefact_info.clone())?;
            let mut buf_writer = BufWriter::new(artefact_file);
            buf_writer
                .write_all(
                    &response
                        .bytes()
                        .await
                        .with_context(|| artefact_info.clone())?,
                )
                .await
                .with_context(|| artefact_info.clone())?;
        }
    } else {
        debug!("the artefact already exists at {}", artefact_path.display());
    }

    // Calculate the downloaded file's hash
    let artefact_file = File::open(&artefact_path)
        .await
        .with_context(|| artefact_info.clone())?;
    let mut stream = ReaderStream::new(artefact_file);
    let mut hasher = Sha512::new();
    while let Some(chunk) = stream.next().await {
        hasher.update(&chunk.with_context(|| artefact_info.clone())?);
    }
    let local_hash: [u8; 64] = hasher.finalize().into();
    let local_hash = hex::encode(local_hash);

    // Verify the file's checksum
    if local_hash != artefact_info.checksum.0 {
        return Err(anyhow!(
            "{}: artefact on local file system has mismatching hash 'sha512:{local_hash}'",
            artefact_info.clone(),
        ));
    }

    Ok((artefact_path, artefact_info))
}

#[instrument]
fn validate_content_type(response: &Response) -> Result<()> {
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .ok_or_else(|| anyhow!("missing Content-Type header"))?;

    if !(content_type == "application/java-archive" || content_type == "application/zip") {
        return Err(anyhow!(
            "the content type must be either a JAR archive or a ZIP file"
        ));
    }

    Ok(())
}

#[instrument]
fn validate_content_length(response: &Response) -> Result<()> {
    let body_length = response.content_length().unwrap_or_else(|| {
        debug!("unknown response body length");
        0
    });

    let content_length: u64 = response
        .headers()
        .get(CONTENT_LENGTH)
        .ok_or_else(|| anyhow!("missing Content-Length header"))
        .and_then(|cl| cl.to_str().map_err(Into::<anyhow::Error>::into))
        .and_then(|cl| cl.parse().map_err(Into::<anyhow::Error>::into))?;

    if body_length != content_length {
        warn!(
            "content-length mismatch: body is {body_length} bytes long, but Content-Type header declares {content_length} bytes"
        );
    }

    Ok(())
}
