use pathgen::{AppPathRole, Env, PathPolicy};

fn print_layout(env: Env) -> Result<(), pathgen::PathgenError> {
    let paths = PathPolicy::new("video-archive-api", env)?;
    println!("env={env}");
    println!(
        "  config  {}",
        paths.dir(AppPathRole::Config)?.path.display()
    );
    println!("  logs    {}", paths.dir(AppPathRole::Logs)?.path.display());
    println!(
        "  runtime {}",
        paths.dir(AppPathRole::Runtime)?.path.display()
    );
    Ok(())
}

fn main() -> Result<(), pathgen::PathgenError> {
    print_layout(Env::Prod)?;
    print_layout(Env::Dev)?;
    print_layout(Env::Test)?;
    Ok(())
}
