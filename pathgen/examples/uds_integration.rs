use std::path::PathBuf;

use pathgen::PathPolicy;

#[derive(Debug)]
struct UdsEndpointConfig {
    app: String,
    socket_name: String,
    socket_path: PathBuf,
}

fn default_socket_config(
    app: &str,
    socket_name: &str,
) -> Result<UdsEndpointConfig, pathgen::PathgenError> {
    let paths = PathPolicy::prod(app)?;
    let socket = paths.socket(socket_name)?;

    Ok(UdsEndpointConfig {
        app: app.to_string(),
        socket_name: socket_name.to_string(),
        socket_path: socket.path,
    })
}

fn main() -> Result<(), pathgen::PathgenError> {
    let config = default_socket_config("video-archive-api", "telemetry.sock")?;
    println!("{config:#?}");
    println!("{}:{}", config.app, config.socket_name);
    println!("socket path: {}", config.socket_path.display());
    Ok(())
}
