use modrinth::USER_AGENT;
use modrinth::business_logic::{
    http_client, load_lockfile, load_manifest, process_manifest, save_lockfile,
};
use modrinth::spec::Spec;
use std::path::PathBuf;
use tracing::{error, instrument, warn};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt};
use tracing_subscriber::{Layer, util::SubscriberInitExt};

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    /// If enabled, force server-only manifests
    #[arg(short, long)]
    server_only: bool,
    /// Specify the Minecraft version (default from environment variable "MINECRAFT_VERSION")
    #[arg(short, long, env = "MINECRAFT_VERSION", default_value = "1.21.1")]
    minecraft_version: String,
    /// Specify the destination for downloaded artefacts (default is the current working directory)
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Specify the path to the manifest file
    #[arg(long)]
    manifest: Option<PathBuf>,
    /// Specify the path to the lockfile
    #[arg(long)]
    lockfile: Option<PathBuf>,
    /// Just update the lockfile, don't download the artifacts
    #[arg(long)]
    no_download: bool,
    /// Fail if no compatible versions are found
    #[arg(long)]
    strict: bool,
}

impl Args {
    fn output(&self) -> Result<PathBuf> {
        let output = match &self.output {
            Some(o) => o.clone(),
            None => std::env::current_dir()?,
        };

        Ok(output)
    }

    fn manifest(&self) -> Result<PathBuf> {
        let manifest = match &self.manifest {
            Some(m) => m.clone(),
            None => std::env::current_dir()?.join("Modrinth.toml"),
        };

        Ok(manifest)
    }

    fn lockfile(&self) -> Result<PathBuf> {
        let lockfile = match &self.lockfile {
            Some(l) => l.clone(),
            None => std::env::current_dir()?.join("Modrinth.lock"),
        };

        Ok(lockfile)
    }
}

fn modrinth_api_token() -> Result<Option<String>> {
    let api_token = match std::env::var("MODRINTH_PAT") {
        Ok(pat) => Some(pat),
        Err(std::env::VarError::NotPresent) => None,
        Err(e) => {
            return Err(Into::<anyhow::Error>::into(e))
                .context("reading the environment variable 'MODRINTH_PAT'");
        }
    };

    Ok(api_token)
}

#[instrument(skip_all)]
async fn inner_main(args: &Args) -> Result<()> {
    let modrinth_api_token = modrinth_api_token().context("loading Modrinth REST API token")?;

    let output_path = args.output()?;
    let manifest_path = args.manifest()?;
    let lockfile_path = args.lockfile()?;

    // Load the manifest file
    let manifest = load_manifest(&manifest_path)
        .await
        .context("loading the manifest file")?;

    // Load the lock file
    let lockfile = load_lockfile(&manifest, &lockfile_path)
        .await
        .context("loading the lockfile")?;

    // Construct the specification.
    let spec = Spec {
        minecraft_version: args.minecraft_version.clone(),
        server_only: args.server_only,
        manifest: manifest.clone(),
        lockfile,
        ..Default::default()
    };

    // Create the HTTP client for accessing the REST API
    let client = http_client(USER_AGENT, modrinth_api_token.as_deref())
        .context("creating the HTTP REST API client")?;

    let lock = process_manifest(&client, &spec, &output_path, args.no_download, args.strict)
        .await
        .context("downloading the specified artefacts")?;

    // Save the lock file
    if let Err(e) = save_lockfile(&lock, &lockfile_path).await {
        warn!("cannot write to lockfile: {e}");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Register tracing and logging facilities
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_filter(EnvFilter::from_default_env()),
        )
        .init();

    // Parse command-line arguments
    let args = Args::parse();

    // Start the program in earnest
    if let Err(e) = inner_main(&args).await {
        error!("{e}");
        Err(e)
    } else {
        Ok(())
    }
}
