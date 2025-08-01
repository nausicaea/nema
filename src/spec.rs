use anyhow::anyhow;
use std::collections::{BTreeMap, HashSet};

use crate::modrinth::{api::MODRINTH_PROD_BASE_URL, model::File};
use reqwest::Url;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Default)]
pub struct Project {
    pub version: Option<String>,
}

impl<'de> Deserialize<'de> for Project {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ProjectHelper {
            String(String),
            Table { version: String },
        }

        match ProjectHelper::deserialize(deserializer)? {
            ProjectHelper::String(s) if s == "*" => Ok(Project { version: None }),
            ProjectHelper::String(s) => Ok(Project { version: Some(s) }),
            ProjectHelper::Table { version } => Ok(Project {
                version: Some(version),
            }),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize)]
pub enum Loader {
    Datapack,
    Fabric,
}

impl std::fmt::Display for Loader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Loader::Datapack => write!(f, "datapack"),
            Loader::Fabric => write!(f, "fabric"),
        }
    }
}

impl<S: AsRef<str>> PartialEq<S> for Loader {
    fn eq(&self, other: &S) -> bool {
        let loader = other.as_ref();
        match self {
            Loader::Datapack if loader == "datapack" => true,
            Loader::Fabric if loader == "fabric" => true,
            _ => false,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Manifest {
    pub datapack: BTreeMap<String, Project>,
    pub fabric: BTreeMap<String, Project>,
}

impl Manifest {
    pub fn new<S, T, I, J>(datapack: I, fabric: I) -> Manifest
    where
        S: AsRef<str>,
        T: AsRef<str>,
        I: IntoIterator<Item = S>,
        J: IntoIterator<Item = T>,
    {
        Manifest {
            datapack: datapack
                .into_iter()
                .map(|s| (s.as_ref().to_string(), Project::default()))
                .collect(),
            fabric: fabric
                .into_iter()
                .map(|s| (s.as_ref().to_string(), Project::default()))
                .collect(),
        }
    }

    pub fn get(&self, loader: Loader) -> impl Iterator<Item = (&String, &Project)> {
        match loader {
            Loader::Datapack => self.datapack.iter(),
            Loader::Fabric => self.fabric.iter(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.datapack.is_empty() && self.fabric.is_empty()
    }
}

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
    pub fn new(project_id: &str, project_slug: &str, version_id: &str, version_number: &str, file: &File) -> anyhow::Result<Self> {
        let sha512_hash =
            file.hashes.sha512.clone().ok_or_else(|| {
                anyhow!("missing SHA-512 file hash in the response from Modrinth",)
            })?;

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
        write!(
            f,
            "{}/{}/{}",
            self.project_id, self.version_id, self.filename
        )
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockfileV1 {
    pub datapack: Vec<Artefact>,
    pub fabric: Vec<Artefact>,
}

impl LockfileV1 {
    pub fn get(&self, loader: Loader) -> &[Artefact] {
        match loader {
            Loader::Datapack => self.datapack.as_slice(),
            Loader::Fabric => self.fabric.as_slice(),
        }
    }

    pub fn index(&self, loader: Loader) -> LockfileIndexV1 {
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

#[derive(Debug)]
pub struct Denylist(Vec<&'static str>);

impl Denylist {
    pub fn contains(&self, v: &str) -> bool {
        self.0.contains(&v)
    }
}

impl Default for Denylist {
    fn default() -> Self {
        Denylist(vec![
            // The quilted fabric API conflicts with fabric API, so it cannot be used
            "qsl", "qvIfYCYJ",
        ])
    }
}

/// It collects all information that influences which artefacts will be downloaded
#[derive(Debug)]
pub struct Spec {
    pub modrinth_api_url: Url,
    pub minecraft_version: String,
    pub server_only: bool,
    pub manifest: Manifest,
    pub lockfile: LockfileV1,
    pub denylist: Denylist,
}

impl Default for Spec {
    fn default() -> Self {
        Spec {
            modrinth_api_url: Url::parse(MODRINTH_PROD_BASE_URL).expect("a valid absolute URL"),
            minecraft_version: "1.21.1".into(),
            server_only: true,
            manifest: Manifest::default(),
            lockfile: LockfileV1::default(),
            denylist: Denylist::default(),
        }
    }
}
