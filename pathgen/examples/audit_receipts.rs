use pathgen::{AppPathRole, PathPolicy};

fn main() -> Result<(), pathgen::PathgenError> {
    let paths = PathPolicy::prod("video-archive-api")?;

    let receipts = [
        paths.config_file("app.toml")?,
        paths.log_file("server.log")?,
        paths.socket("api.sock")?,
        paths.dir(AppPathRole::Secrets)?,
    ];

    for receipt in receipts {
        println!("{}", receipt.audit_line());
    }

    Ok(())
}
