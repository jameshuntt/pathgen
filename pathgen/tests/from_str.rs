use pathgen::{AppPathRole, Env, StandardPath};

#[test]
fn env_from_str_aliases() {
    assert_eq!("prod".parse::<Env>().unwrap(), Env::Prod);
    assert_eq!("production".parse::<Env>().unwrap(), Env::Prod);
    assert_eq!("dev".parse::<Env>().unwrap(), Env::Dev);
    assert_eq!("development".parse::<Env>().unwrap(), Env::Dev);
    assert_eq!("test".parse::<Env>().unwrap(), Env::Test);
    assert_eq!("testing".parse::<Env>().unwrap(), Env::Test);
}

#[test]
fn standard_path_from_str_aliases() {
    assert_eq!("/".parse::<StandardPath>().unwrap(), StandardPath::Root);
    assert_eq!("etc".parse::<StandardPath>().unwrap(), StandardPath::Etc);
    assert_eq!(
        "/var/log".parse::<StandardPath>().unwrap(),
        StandardPath::VarLog
    );
    assert_eq!(
        "var_cache".parse::<StandardPath>().unwrap(),
        StandardPath::VarCache
    );
    assert_eq!(
        "usr_sbin".parse::<StandardPath>().unwrap(),
        StandardPath::UsrSbin
    );
}

#[test]
fn app_path_role_from_str_aliases() {
    assert_eq!(
        "config".parse::<AppPathRole>().unwrap(),
        AppPathRole::Config
    );
    assert_eq!("conf".parse::<AppPathRole>().unwrap(), AppPathRole::Config);
    assert_eq!("log".parse::<AppPathRole>().unwrap(), AppPathRole::Logs);
    assert_eq!("run".parse::<AppPathRole>().unwrap(), AppPathRole::Runtime);
    assert_eq!("tmp".parse::<AppPathRole>().unwrap(), AppPathRole::Temp);
    assert_eq!("tls".parse::<AppPathRole>().unwrap(), AppPathRole::Certs);
}

#[test]
fn unknown_names_error() {
    assert!("wat".parse::<StandardPath>().is_err());
    assert!("wat".parse::<AppPathRole>().is_err());
    assert!("wat".parse::<Env>().is_err());
}
