use crate::modrinth::api::MODRINTH_PROD_BASE_URL;
use denylist::Denylist;
use lockfile::LockfileV1;
use manifest::Manifest;
use reqwest::Url;

pub mod denylist;
pub mod lockfile;
pub mod manifest;

/// It collects all information that influences which artefacts will be downloaded
#[derive(Debug)]
pub struct Spec {
    pub modrinth_api_url: Url,
    pub server_only: bool,
    pub manifest: Manifest,
    pub lockfile: LockfileV1,
    pub denylist: Denylist,
}

impl Default for Spec {
    fn default() -> Self {
        Spec {
            modrinth_api_url: Url::parse(MODRINTH_PROD_BASE_URL).expect("a valid absolute URL"),
            server_only: true,
            manifest: Manifest::default(),
            lockfile: LockfileV1::default(),
            denylist: Denylist::default(),
        }
    }
}
