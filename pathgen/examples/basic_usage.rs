use pathgen::{AppPathRole, PathPolicy};

fn main() -> Result<(), pathgen::PathgenError> {
    let paths = PathPolicy::prod("video-archive-api")?;

    let config = paths.file(AppPathRole::Config, "app.toml")?;
    let log = paths.file(AppPathRole::Logs, "server.log")?;
    let socket = paths.socket("api.sock")?;

    println!("config: {}", config.path.display());
    println!("log:    {}", log.path.display());
    println!("socket: {}", socket.path.display());

    Ok(())
}
