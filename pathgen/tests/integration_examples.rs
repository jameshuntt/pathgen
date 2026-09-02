use pathgen::{AppPathRole, PathPolicy};

#[test]
fn logging_integration_uses_policy_not_literal_path() {
    fn default_log_file(app: &str) -> Result<String, pathgen::PathgenError> {
        let paths = PathPolicy::prod(app)?;
        Ok(paths.log_file("server.log")?.path.display().to_string())
    }

    assert_eq!(
        default_log_file("archive-api").unwrap(),
        "/var/log/archive-api/server.log"
    );
}

#[test]
fn uds_integration_uses_runtime_socket() {
    fn default_socket_path(app: &str, socket: &str) -> Result<String, pathgen::PathgenError> {
        let paths = PathPolicy::prod(app)?;
        Ok(paths.socket(socket)?.path.display().to_string())
    }

    assert_eq!(
        default_socket_path("archive-api", "telemetry.sock").unwrap(),
        "/run/archive-api/telemetry.sock"
    );
}

#[test]
fn systemd_builder_can_use_policy_outputs() {
    let paths = PathPolicy::prod("archive-api").unwrap();

    let working_dir = paths.dir(AppPathRole::ProjectRoot).unwrap().path;
    let config = paths.config_file("app.toml").unwrap().path;
    let socket = paths.socket("api.sock").unwrap().path;

    let unit = format!(
        "[Service]\nWorkingDirectory={}\nEnvironment=ARCHIVE_CONFIG={}\nEnvironment=ARCHIVE_SOCKET={}\n",
        working_dir.display(),
        config.display(),
        socket.display(),
    );

    assert!(unit.contains("WorkingDirectory=/opt/archive-api"));
    assert!(unit.contains("ARCHIVE_CONFIG=/etc/archive-api/app.toml"));
    assert!(unit.contains("ARCHIVE_SOCKET=/run/archive-api/api.sock"));
}

#[test]
fn dry_run_manifest_can_feed_audit_output() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    let lines: Vec<String> = paths
        .dry_run_manifest()
        .unwrap()
        .into_iter()
        .map(|receipt| receipt.audit_line())
        .collect();

    assert_eq!(lines.len(), 11);
    assert!(lines.iter().any(|line| line.contains("role=config")));
    assert!(lines.iter().any(|line| line.contains("role=runtime")));
    assert!(lines.iter().any(|line| line.contains("role=secrets")));
}
