use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct FileHashes {
    pub sha512: Option<String>,
}

#[derive(Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct File {
    pub hashes: FileHashes,
    pub url: String,
    pub filename: String,
    pub primary: bool,
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File")
            .field("filename", &self.filename)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct Dependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VersionType {
    Release,
    Beta,
    Alpha,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VersionStatus {
    Listed,
    Archived,
    Draft,
    Unlisted,
    Scheduled,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct Version {
    pub loaders: Vec<String>,
    pub id: String,
    pub project_id: String,
    pub version_number: String,
    pub date_published: chrono::DateTime<chrono::Utc>,
    pub files: Vec<File>,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PlatformRequirement {
    Required,
    Optional,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct Project {
    pub slug: String,
    pub id: String,
    pub client_side: PlatformRequirement,
    pub server_side: PlatformRequirement,
}
