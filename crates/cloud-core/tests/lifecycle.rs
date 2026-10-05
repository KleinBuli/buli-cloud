use cloud_core::{CloudCore, instances::instance::InstanceStatus, util::software::softwaremanager::ServerSoftware};
use std::{fs, io::ErrorKind};

async fn fixture() -> (tempfile::TempDir, CloudCore) {
    let root = tempfile::tempdir().unwrap();
    let core = CloudCore::new(root.path()).await.unwrap();
    fs::create_dir_all(core.cache_path()).unwrap();
    let version = core.config_manager().config().fallback_minecraft_version();
    fs::write(
        core.cache_path().join(format!("paper-{version}.jar")),
        b"test fixture, not executable",
    )
    .unwrap();
    core.initialize().await.unwrap();
    (root, core)
}

#[tokio::test]
async fn restart_preserves_assignments_maintenance_and_software() {
    let (root, core) = fixture().await;
    core.create_group("lobby").await.unwrap();
    fs::write(core.cache_path().join("vanilla-1.20.6.jar"), b"vanilla").unwrap();
    fs::write(core.cache_path().join("velocity.jar"), b"velocity").unwrap();
    core.create_template("lobby", "old", ServerSoftware::Vanilla, Some("1.20.6".into()))
        .await
        .unwrap();
    core.create_template("lobby", "proxy", ServerSoftware::Velocity, None)
        .await
        .unwrap();
    core.set_group_maintenance("lobby", true).await.unwrap();
    drop(core);
    let core = CloudCore::new(root.path()).await.unwrap();
    core.initialize().await.unwrap();
    let group = core.group_manager().get_group("lobby").await.unwrap();
    assert_eq!(group.template_names(), &["global", "old", "proxy"]);
    assert!(group.maintenance());
    let vanilla = core.template_manager().get_template("lobby", "old").await.unwrap();
    assert_eq!(vanilla.server_software(), &ServerSoftware::Vanilla);
    assert_eq!(vanilla.minecraft_version().as_deref(), Some("1.20.6"));
    let proxy = core.template_manager().get_template("lobby", "proxy").await.unwrap();
    assert_eq!(proxy.server_software(), &ServerSoftware::Velocity);
    assert_eq!(proxy.minecraft_version(), &None);
    core.delete_template("lobby", "old").await.unwrap();
    let reloaded = CloudCore::new(root.path()).await.unwrap();
    reloaded.initialize().await.unwrap();
    assert_eq!(
        reloaded.group_manager().get_group("lobby").await.unwrap().template_names(),
        &["global", "proxy"]
    );
}

#[tokio::test]
async fn instances_copy_their_group_template_without_starting_a_process() {
    let (_root, core) = fixture().await;
    for group in ["a", "b"] {
        core.create_group(group).await.unwrap();
        fs::write(core.templates_path().join(group).join("global/marker.txt"), group).unwrap();
    }
    let instance = core.create_instance_from_template("b", "global").await.unwrap();
    assert_eq!(instance.status(), &InstanceStatus::Stopped);
    assert_eq!(
        fs::read_to_string(core.running_path().join(instance.id()).join("marker.txt")).unwrap(),
        "b"
    );
    assert!(!core.running_path().join(instance.id()).join(".bulicloud-template.toml").exists());
    assert!(core.running_path().join(instance.id()).join("paper-1.21.11.jar").exists());
    core.remove_instance(instance.id()).await.unwrap();
    assert!(!core.instance_manager().exists(instance.id()).await);
    assert!(!core.running_path().join(instance.id()).exists());
}

#[tokio::test]
async fn failed_preparation_does_not_leave_instance_or_partial_directory() {
    let (_root, core) = fixture().await;
    core.create_group("a").await.unwrap();
    // Force copy failure after registration, without launching any real server.
    fs::remove_dir(core.running_path()).unwrap();
    fs::write(core.running_path(), b"not a directory").unwrap();
    assert!(core.create_instance_from_template("a", "global").await.is_err());
    assert!(core.instance_manager().instances_list().await.is_empty());
}

#[tokio::test]
async fn maintenance_and_reference_checks_guard_lifecycle() {
    let (_root, core) = fixture().await;
    core.create_group("a").await.unwrap();
    core.create_template("a", "custom", ServerSoftware::Paper, Some("1.21.11".into()))
        .await
        .unwrap();
    let instance = core.create_instance_from_template("a", "custom").await.unwrap();
    assert_eq!(core.remove_group("a").await.unwrap_err().kind(), ErrorKind::ResourceBusy);
    assert_eq!(
        core.delete_template("a", "custom").await.unwrap_err().kind(),
        ErrorKind::ResourceBusy
    );
    assert_eq!(
        core.delete_template("a", "global").await.unwrap_err().kind(),
        ErrorKind::ResourceBusy
    );
    core.set_group_maintenance("a", true).await.unwrap();
    assert_eq!(
        core.start_instance(instance.id()).await.unwrap_err().kind(),
        ErrorKind::ResourceBusy
    );
    assert_eq!(
        core.create_instance_from_template("a", "custom").await.unwrap_err().kind(),
        ErrorKind::ResourceBusy
    );
    core.remove_instance(instance.id()).await.unwrap();
    core.delete_template("a", "custom").await.unwrap();
    core.remove_group("a").await.unwrap();
    assert!(!core.templates_path().join("a").exists());
    assert!(core.template_manager().templates_list("a").await.is_empty());
    assert!(core.group_manager().get_group("a").await.is_none());
}

#[tokio::test]
async fn failed_config_commit_rolls_back_template_creation_and_deletion() {
    let (_root, core) = fixture().await;
    core.create_group("a").await.unwrap();
    core.create_template("a", "existing", ServerSoftware::Paper, Some("1.21.11".into()))
        .await
        .unwrap();
    let config = core.config_path().join("groups.toml");
    fs::rename(&config, core.config_path().join("groups.backup")).unwrap();
    fs::create_dir(&config).unwrap();
    assert!(
        core.create_template("a", "new", ServerSoftware::Paper, Some("1.21.11".into()))
            .await
            .is_err()
    );
    assert!(!core.templates_path().join("a/new").exists());
    assert!(core.template_manager().get_template("a", "new").await.is_none());
    assert!(core.delete_template("a", "existing").await.is_err());
    assert!(core.templates_path().join("a/existing").is_dir());
    assert!(core.template_manager().get_template("a", "existing").await.is_some());
    assert_eq!(
        core.group_manager().get_group("a").await.unwrap().template_names(),
        &["global", "existing"]
    );
    assert!(core.set_group_maintenance("a", true).await.is_err());
    assert!(!core.group_manager().get_group("a").await.unwrap().maintenance());
}

#[tokio::test]
async fn failed_group_deletion_restores_files_and_memory() {
    let (_root, core) = fixture().await;
    core.create_group("a").await.unwrap();
    let config = core.config_path().join("groups.toml");
    fs::remove_file(&config).unwrap();
    fs::create_dir(&config).unwrap();
    assert!(core.remove_group("a").await.is_err());
    assert!(core.templates_path().join("a/global").is_dir());
    assert!(core.group_manager().get_group("a").await.is_some());
}

#[tokio::test]
async fn missing_software_and_invalid_versions_do_not_create_templates() {
    let (_root, core) = fixture().await;
    core.create_group("a").await.unwrap();
    assert_eq!(
        core.create_template("a", "missing", ServerSoftware::Paper, Some("0.0".into()))
            .await
            .unwrap_err()
            .kind(),
        ErrorKind::NotFound
    );
    assert_eq!(
        core.create_template("a", "invalid", ServerSoftware::Paper, None)
            .await
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidInput
    );
    assert!(!core.templates_path().join("a/missing").exists());
    assert!(!core.templates_path().join("a/invalid").exists());
    assert_eq!(core.group_manager().get_group("a").await.unwrap().template_names(), &["global"]);
}

#[tokio::test]
async fn stale_runtime_directories_are_preserved_and_ids_are_not_reused() {
    let (root, core) = fixture().await;
    core.create_group("a").await.unwrap();
    let old = core.create_instance_from_template("a", "global").await.unwrap();
    fs::write(core.running_path().join(old.id()).join("world.txt"), b"keep").unwrap();
    core.shutdown().await.unwrap();
    assert_eq!(core.create_group("b").await.unwrap_err().kind(), ErrorKind::ResourceBusy);
    drop(core);
    let core = CloudCore::new(root.path()).await.unwrap();
    core.initialize().await.unwrap();
    let new = core.create_instance_from_template("a", "global").await.unwrap();
    assert_eq!(new.id(), "a-2");
    assert_eq!(fs::read(core.running_path().join("a-1/world.txt")).unwrap(), b"keep");
}

#[tokio::test]
async fn legacy_metadata_is_inferred_from_the_actual_jar() {
    let (root, core) = fixture().await;
    core.create_group("a").await.unwrap();
    fs::write(core.cache_path().join("vanilla-1.20.6.jar"), b"vanilla").unwrap();
    core.create_template("a", "old", ServerSoftware::Vanilla, Some("1.20.6".into()))
        .await
        .unwrap();
    let config = core.templates_path().join("a/old/.bulicloud-template.toml");
    fs::remove_file(&config).unwrap();
    drop(core);
    let core = CloudCore::new(root.path()).await.unwrap();
    core.initialize().await.unwrap();
    assert_eq!(
        core.template_manager().get_template("a", "old").await.unwrap().server_software(),
        &ServerSoftware::Vanilla
    );
    assert!(config.exists());
}

#[tokio::test]
async fn ambiguous_legacy_metadata_is_rejected() {
    let (root, core) = fixture().await;
    core.create_group("a").await.unwrap();
    let path = core.templates_path().join("a/global");
    fs::remove_file(path.join(".bulicloud-template.toml")).unwrap();
    fs::write(path.join("velocity.jar"), b"proxy").unwrap();
    drop(core);
    let core = CloudCore::new(root.path()).await.unwrap();
    assert_eq!(core.initialize().await.unwrap_err().kind(), ErrorKind::InvalidData);
    assert!(!path.join(".bulicloud-template.toml").exists());
}

#[tokio::test]
async fn path_components_cannot_escape_managed_directories() {
    let (_root, core) = fixture().await;
    for name in ["..", "../outside", "C:\\outside", "a/b", "CON", "name."] {
        assert_eq!(core.create_group(name).await.unwrap_err().kind(), ErrorKind::InvalidInput);
    }
    core.create_group("a").await.unwrap();
    assert_eq!(
        core.create_template("a", "../outside", ServerSoftware::Paper, Some("1.21.11".into()))
            .await
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidInput
    );
}

#[tokio::test]
async fn concurrent_template_mutations_survive_reload() {
    let (root, core) = fixture().await;
    core.create_group("a").await.unwrap();
    let (first, second) = tokio::join!(
        core.create_template("a", "one", ServerSoftware::Paper, Some("1.21.11".into())),
        core.create_template("a", "two", ServerSoftware::Paper, Some("1.21.11".into()))
    );
    first.unwrap();
    second.unwrap();
    let reloaded = CloudCore::new(root.path()).await.unwrap();
    reloaded.initialize().await.unwrap();
    let group = reloaded.group_manager().get_group("a").await.unwrap();
    assert_eq!(group.template_names().len(), 3);
    assert!(group.template_names().iter().any(|n| n == "one"));
    assert!(group.template_names().iter().any(|n| n == "two"));
}

#[tokio::test]
async fn changes_before_initialization_cannot_overwrite_existing_configuration() {
    let (root, core) = fixture().await;
    core.create_group("existing").await.unwrap();
    let fresh = CloudCore::new(root.path()).await.unwrap();
    assert_eq!(fresh.create_group("new").await.unwrap_err().kind(), ErrorKind::ResourceBusy);
    fresh.initialize().await.unwrap();
    assert!(fresh.group_manager().get_group("existing").await.is_some());
}

#[tokio::test]
async fn group_names_cannot_alias_the_same_windows_directory() {
    let (_root, core) = fixture().await;
    core.create_group("Lobby").await.unwrap();
    assert_eq!(core.create_group("lobby").await.unwrap_err().kind(), ErrorKind::AlreadyExists);
    assert_eq!(core.group_manager().groups().await.len(), 1);
}

#[tokio::test]
async fn empty_legacy_global_template_gets_metadata_and_the_fallback_jar() {
    let (root, core) = fixture().await;
    core.create_group("a").await.unwrap();
    let path = core.templates_path().join("a/global");
    fs::remove_file(path.join(".bulicloud-template.toml")).unwrap();
    fs::remove_file(path.join("paper-1.21.11.jar")).unwrap();
    drop(core);
    let core = CloudCore::new(root.path()).await.unwrap();
    core.initialize().await.unwrap();
    assert!(path.join(".bulicloud-template.toml").is_file());
    assert!(path.join("paper-1.21.11.jar").is_file());
}
