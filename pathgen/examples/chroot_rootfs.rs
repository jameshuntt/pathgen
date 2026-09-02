use pathgen::{AppPathRole, PathPolicy};

fn main() -> Result<(), pathgen::PathgenError> {
    let paths = PathPolicy::prod("video-archive-api")?.with_root("/srv/rootfs");

    println!("config: {}", paths.config_file("app.toml")?.path.display());
    println!("logs:   {}", paths.log_file("server.log")?.path.display());
    println!("socket: {}", paths.socket("api.sock")?.path.display());
    println!(
        "units:  {}",
        paths.dir(AppPathRole::SystemdUnits)?.path.display()
    );

    Ok(())
}
