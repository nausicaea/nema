use anyhow::anyhow;
use std::collections::HashSet;

use crate::modrinth::model::File;
use serde::{Deserialize, Serialize};

use super::manifest::Loader;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sha512Checksum(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Artefact {
    pub project_id: String,
    pub project_slug: String,
    pub version_id: String,
    pub version_number: String,
    pub filename: String,
    pub checksum: Sha512Checksum,
}

impl Artefact {
    pub fn new(
        project_id: &str,
        project_slug: &str,
        version_id: &str,
        version_number: &str,
        file: &File,
    ) -> anyhow::Result<Self> {
        let sha512_hash = file
            .hashes
            .sha512
            .clone()
            .ok_or_else(|| anyhow!("missing SHA-512 file hash in the response from Modrinth",))?;

        Ok(Artefact {
            project_id: project_id.to_string(),
            project_slug: project_slug.to_string(),
            version_id: version_id.to_string(),
            version_number: version_number.to_string(),
            filename: file.filename.clone(),
            checksum: Sha512Checksum(sha512_hash),
        })
    }
}

impl std::fmt::Display for Artefact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.project_id, self.version_id, self.filename)
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockfileV1 {
    pub minecraft_version: String,
    pub datapack: Vec<Artefact>,
    pub fabric: Vec<Artefact>,
}

impl LockfileV1 {
    pub fn artefacts(&self) -> impl Iterator<Item = &Artefact> {
        self.datapack.iter().chain(self.fabric.iter())
    }

    pub fn get(&self, loader: Loader) -> &[Artefact] {
        match loader {
            Loader::Datapack => self.datapack.as_slice(),
            Loader::Fabric => self.fabric.as_slice(),
        }
    }

    pub fn index(&self, loader: Loader) -> LockfileIndexV1<'_> {
        match loader {
            Loader::Datapack => LockfileIndexV1(
                self.datapack
                    .iter()
                    .map(|a| (a.project_id.as_str(), a.version_id.as_str()))
                    .collect(),
            ),
            Loader::Fabric => LockfileIndexV1(
                self.fabric
                    .iter()
                    .map(|a| (a.project_id.as_str(), a.version_id.as_str()))
                    .collect(),
            ),
        }
    }
}

#[derive(Debug)]
pub struct LockfileIndexV1<'a>(HashSet<(&'a str, &'a str)>);

impl<'a> LockfileIndexV1<'a> {
    /// Returns `true` if the lockfile is empty
    pub fn contains(&self, v: (&str, &str)) -> bool {
        if self.0.is_empty() {
            return true;
        }

        self.0.contains(&v)
    }
}
