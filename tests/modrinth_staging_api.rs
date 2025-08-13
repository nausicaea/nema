use modrinth::{
    USER_AGENT,
    business_logic::{http_client, process_manifest},
    modrinth::api::MODRINTH_STAGING_BASE_URL,
    spec::{
        Spec,
        manifest::{Manifest, Project},
    },
};
use reqwest::{Client, Url};
use rstest::{fixture, rstest};
use tempfile::TempDir;

#[fixture]
fn client() -> Client {
    http_client(USER_AGENT, None).unwrap()
}

#[fixture]
fn modrinth_staging_api() -> Url {
    Url::parse(MODRINTH_STAGING_BASE_URL).unwrap()
}

#[fixture]
fn tempdir() -> TempDir {
    tempfile::tempdir().unwrap()
}

#[rstest]
#[tokio::test]
async fn lockfile_generation(client: Client, modrinth_staging_api: Url, #[from(tempdir)] output: TempDir) {
    let spec = Spec {
        modrinth_api_url: modrinth_staging_api,
        manifest: Manifest {
            minecraft_version: "1.8.9".into(),
            fabric: vec!["soundfix"]
                .into_iter()
                .map(|p| (p.to_string(), Project::default()))
                .collect(),
            ..Default::default()
        },
        ..Default::default()
    };

    let lockfile = process_manifest(&client, &spec, output.path(), true, true)
        .await
        .unwrap();

    assert_eq!(lockfile.fabric.len(), 1);
}

#[rstest]
#[tokio::test]
async fn project_denylisting(client: Client, #[from(tempdir)] output: TempDir) {
    let spec = Spec {
        manifest: Manifest {
            minecraft_version: "1.21.8".into(),
            fabric: vec!["dynamic-lights"]
                .into_iter()
                .map(|p| (p.to_string(), Project::default()))
                .collect(),
            ..Default::default()
        },
        ..Default::default()
    };

    let lockfile = process_manifest(&client, &spec, output.path(), true, true)
        .await
        .unwrap();

    assert!(!lockfile.fabric.iter().any(|a| spec.denylist.contains(&a.project_id)));
}

#[rstest]
#[tokio::test]
async fn lockfile_includes_dependencies(client: Client, #[from(tempdir)] output: TempDir) {
    let spec = Spec {
        manifest: Manifest {
            minecraft_version: "1.21.8".into(),
            fabric: vec!["dynamic-lights"]
                .into_iter()
                .map(|p| (p.to_string(), Project::default()))
                .collect(),
            ..Default::default()
        },
        ..Default::default()
    };

    let lockfile = process_manifest(&client, &spec, output.path(), true, true)
        .await
        .unwrap();

    assert_eq!(lockfile.fabric.len(), 2);
}

#[rstest]
#[tokio::test]
async fn lockfile_works_for_datapacks(client: Client, #[from(tempdir)] output: TempDir) {
    let spec = Spec {
        manifest: Manifest {
            minecraft_version: "1.21.1".into(),
            datapack: vec!["veinminer"]
                .into_iter()
                .map(|p| (p.to_string(), Project::default()))
                .collect(),
            ..Default::default()
        },
        ..Default::default()
    };

    let lockfile = process_manifest(&client, &spec, output.path(), true, true)
        .await
        .unwrap();

    assert_eq!(lockfile.datapack.len(), 1, "{lockfile:?}");
}
