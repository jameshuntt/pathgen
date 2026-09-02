use pathgen::PathPolicy;

fn main() -> Result<(), pathgen::PathgenError> {
    let paths = PathPolicy::prod("video-archive-api")?;

    for receipt in paths.dry_run_manifest()? {
        println!("{}", receipt.audit_line());
    }

    Ok(())
}
