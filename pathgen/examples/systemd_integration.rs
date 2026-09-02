use pathgen::{AppPathRole, PathPolicy};

fn build_systemd_unit(app: &str) -> Result<String, pathgen::PathgenError> {
    let paths = PathPolicy::prod(app)?;

    let working_dir = paths.dir(AppPathRole::ProjectRoot)?;
    let config = paths.config_file("app.toml")?;
    let log = paths.log_file("server.log")?;
    let socket = paths.socket("api.sock")?;

    Ok(format!(
        "[Unit]\nDescription={app}\n\n[Service]\nWorkingDirectory={}\nExecStart={}/bin/{app}\nEnvironment=APP_CONFIG={}\nEnvironment=APP_LOG={}\nEnvironment=APP_SOCKET={}\nRestart=on-failure\n\n[Install]\nWantedBy=multi-user.target\n",
        working_dir.path.display(),
        working_dir.path.display(),
        config.path.display(),
        log.path.display(),
        socket.path.display(),
    ))
}

fn main() -> Result<(), pathgen::PathgenError> {
    let unit = build_systemd_unit("video-archive-api")?;
    println!("{unit}");
    Ok(())
}
