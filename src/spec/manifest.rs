use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer};

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

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub minecraft_version: String,
    pub datapack: BTreeMap<String, Project>,
    pub fabric: BTreeMap<String, Project>,
}

impl Manifest {
    pub fn new<S, T, U, I, J>(minecraft_version: U, datapack: I, fabric: I) -> Manifest
    where
        S: AsRef<str>,
        T: AsRef<str>,
        U: AsRef<str>,
        I: IntoIterator<Item = S>,
        J: IntoIterator<Item = T>,
    {
        Manifest {
            minecraft_version: minecraft_version.as_ref().to_string(),
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

impl Default for Manifest {
    fn default() -> Self {
        Manifest {
            minecraft_version: "1.21.1".to_string(),
            datapack: BTreeMap::default(),
            fabric: BTreeMap::default(),
        }
    }
}

