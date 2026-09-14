use auth::{LocalUserStore, TenantId};
use compiler::compile_file;
use data::migrations::{MigrationState, apply, status, verify};
use std::env;
use std::path::PathBuf;
use std::time::Duration;

use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
enum CliError {
    Usage(String),
    Io(io::Error),
    ParseInt(std::num::ParseIntError),
    Utf8(std::str::Utf8Error),
    Compile(compiler::CompileError),
    Migration(data::migrations::MigrationError),
    Auth(auth::AuthError),
}
impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }
}
impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => f.write_str(message),
            Self::Io(source) => write!(f, "I/O error: {source}"),
            Self::ParseInt(source) => write!(f, "invalid integer: {source}"),
            Self::Utf8(source) => write!(f, "invalid UTF-8: {source}"),
            Self::Compile(source) => write!(f, "{source}"),
            Self::Migration(source) => write!(f, "{source}"),
            Self::Auth(source) => write!(f, "{source}"),
        }
    }
}
impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Usage(_) => None,
            Self::Io(source) => Some(source),
            Self::ParseInt(source) => Some(source),
            Self::Utf8(source) => Some(source),
            Self::Compile(source) => Some(source),
            Self::Migration(source) => Some(source),
            Self::Auth(source) => Some(source),
        }
    }
}
impl From<io::Error> for CliError {
    fn from(v: io::Error) -> Self {
        Self::Io(v)
    }
}
impl From<std::num::ParseIntError> for CliError {
    fn from(v: std::num::ParseIntError) -> Self {
        Self::ParseInt(v)
    }
}
impl From<std::str::Utf8Error> for CliError {
    fn from(v: std::str::Utf8Error) -> Self {
        Self::Utf8(v)
    }
}
impl From<compiler::CompileError> for CliError {
    fn from(v: compiler::CompileError) -> Self {
        Self::Compile(v)
    }
}
impl From<data::migrations::MigrationError> for CliError {
    fn from(v: data::migrations::MigrationError) -> Self {
        Self::Migration(v)
    }
}
impl From<auth::AuthError> for CliError {
    fn from(v: auth::AuthError) -> Self {
        Self::Auth(v)
    }
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), CliError> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("check") => {
            let path = args
                .next()
                .ok_or_else(|| CliError::usage("Usage: velran-cli check <app.vrn>"))?;
            if args.next().is_some() {
                return Err(CliError::usage("Usage: velran-cli check <app.vrn>"));
            }
            let p = compile_file(path)?;
            println!(
                "check passed: {} model(s), {} query(s), {} page(s), {} action(s), {} route(s)",
                p.models.len(),
                p.queries.len(),
                p.pages.len(),
                p.actions.len(),
                p.routes.len()
            );
        }
        Some("migrate") => migrate_command(args).await?,
        Some("auth") => auth_command(args).await?,
        _ => print_usage_and_fail()?,
    }
    Ok(())
}

#[derive(Debug)]
struct MigrationArgs {
    dir: PathBuf,
    db_url_file: PathBuf,
    allow_insecure_db: bool,
    lock_timeout_secs: u64,
}

async fn migrate_command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let action = args
        .next()
        .ok_or_else(|| CliError::usage("missing migration action: status | verify | apply"))?;
    if !matches!(action.as_str(), "status" | "verify" | "apply") {
        return Err(CliError::usage(format!(
            "unknown migration action `{action}`"
        )));
    }
    let parsed = parse_migration_args(args)?;
    let db_url = read_secret_file(&parsed.db_url_file)?;
    match action.as_str() {
        "status" => {
            for item in status(&db_url, &parsed.dir, parsed.allow_insecure_db).await? {
                let state = match item.state {
                    MigrationState::Applied => "applied",
                    MigrationState::Pending => "pending",
                };
                println!("{:04} {:8} {}", item.version, state, item.name);
            }
        }
        "verify" => {
            verify(&db_url, &parsed.dir, parsed.allow_insecure_db).await?;
            println!("migration history verified");
        }
        "apply" => {
            let changed = apply(
                &db_url,
                &parsed.dir,
                parsed.allow_insecure_db,
                Duration::from_secs(parsed.lock_timeout_secs),
            )
            .await?;
            if changed.is_empty() {
                println!("no pending migrations");
            } else {
                for version in changed {
                    println!("applied migration {version:04}");
                }
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn parse_migration_args(mut args: impl Iterator<Item = String>) -> Result<MigrationArgs, CliError> {
    let mut dir = None;
    let mut db_url_file = None;
    let mut allow_insecure_db = false;
    let mut lock_timeout_secs = 30u64;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dir" => {
                dir = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| CliError::usage("--dir requires a path"))?,
                ))
            }
            "--db-url-file" => {
                db_url_file =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        CliError::usage("--db-url-file requires a path")
                    })?))
            }
            "--allow-insecure-db" => allow_insecure_db = true,
            "--lock-timeout-secs" => {
                lock_timeout_secs = args
                    .next()
                    .ok_or_else(|| CliError::usage("--lock-timeout-secs requires a number"))?
                    .parse()?
            }
            _ => return Err(CliError::usage(format!("unknown migration option `{arg}`"))),
        }
    }
    if lock_timeout_secs == 0 {
        return Err(CliError::usage("--lock-timeout-secs must be > 0"));
    }
    Ok(MigrationArgs {
        dir: dir.ok_or_else(|| CliError::usage("missing --dir <migrations>"))?,
        db_url_file: db_url_file.ok_or_else(|| CliError::usage("missing --db-url-file <path>"))?,
        allow_insecure_db,
        lock_timeout_secs,
    })
}

fn read_secret_file(path: &std::path::Path) -> Result<String, CliError> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CliError::usage(format!(
            "secret URL file `{}` must be a regular non-symlink file",
            path.display()
        )));
    }
    if metadata.len() > 16 * 1024 {
        return Err(CliError::usage("secret URL file is too large"));
    }
    let value = std::fs::read_to_string(path)?.trim().to_string();
    if value.is_empty() || value.contains('\n') || value.contains('\r') {
        return Err(CliError::usage(
            "secret URL file must contain one non-empty line",
        ));
    }
    Ok(value)
}

async fn auth_command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let action = args
        .next()
        .ok_or_else(|| CliError::usage("missing auth action"))?;
    let mut db_url_file = None;
    let mut username = None;
    let mut password_file = None;
    let mut roles = Vec::new();
    let mut memberships = Vec::new();
    let mut recovery_count = 8usize;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db-url-file" => {
                db_url_file =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        CliError::usage("--db-url-file requires a path")
                    })?))
            }
            "--username" => {
                username = Some(
                    args.next()
                        .ok_or_else(|| CliError::usage("--username requires a value"))?,
                )
            }
            "--password-file" => {
                password_file =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        CliError::usage("--password-file requires a path")
                    })?))
            }
            "--role" => roles.push(
                args.next()
                    .ok_or_else(|| CliError::usage("--role requires a value"))?,
            ),
            "--tenant" => {
                let raw = args
                    .next()
                    .ok_or_else(|| CliError::usage("--tenant requires a value"))?;
                memberships.push(TenantId::parse(&raw)?);
            }
            "--recovery-count" => {
                recovery_count = args
                    .next()
                    .ok_or_else(|| CliError::usage("--recovery-count requires a number"))?
                    .parse()?
            }
            _ => return Err(CliError::usage(format!("unknown auth option `{arg}`"))),
        }
    }
    let db_url = read_secret_file(
        &db_url_file.ok_or_else(|| CliError::usage("missing --db-url-file <path>"))?,
    )?;
    let store = LocalUserStore::connect_sqlite(&db_url)
        .await
        .map_err(|_| CliError::usage("failed to open local auth DB"))?;
    match action.as_str() {
        "init" => {
            store
                .initialize()
                .await
                .map_err(|_| CliError::usage("failed to initialize local auth DB"))?;
            println!("local auth store initialized");
        }
        "user-add" => {
            store
                .ensure_ready()
                .await
                .map_err(|_| CliError::usage("local auth DB is not initialized"))?;
            let u = username.ok_or_else(|| CliError::usage("missing --username"))?;
            let pw = read_password_file(
                &password_file.ok_or_else(|| CliError::usage("missing --password-file"))?,
            )?;
            if roles.is_empty() {
                roles.push("User".into());
            }
            store
                .create_user(&u, &pw, &roles)
                .await
                .map_err(|_| CliError::usage("failed to create user"))?;
            if !memberships.is_empty() {
                store
                    .set_memberships(&u, &memberships)
                    .await
                    .map_err(|_| CliError::usage("failed to set tenant memberships"))?;
            }
            println!("created user {u}");
        }
        "password-set" => {
            store
                .ensure_ready()
                .await
                .map_err(|_| CliError::usage("local auth DB is not initialized"))?;
            let u = username.ok_or_else(|| CliError::usage("missing --username"))?;
            let pw = read_password_file(
                &password_file.ok_or_else(|| CliError::usage("missing --password-file"))?,
            )?;
            store
                .set_password(&u, &pw)
                .await
                .map_err(|_| CliError::usage("failed to change password"))?;
            println!("password changed for {u}");
        }
        "disable" => {
            let u = username.ok_or_else(|| CliError::usage("missing --username"))?;
            store
                .set_disabled(&u, true)
                .await
                .map_err(|_| CliError::usage("failed to disable user"))?;
            println!("disabled user {u}");
        }
        "enable" => {
            let u = username.ok_or_else(|| CliError::usage("missing --username"))?;
            store
                .set_disabled(&u, false)
                .await
                .map_err(|_| CliError::usage("failed to enable user"))?;
            println!("enabled user {u}");
        }
        "roles-set" => {
            let u = username.ok_or_else(|| CliError::usage("missing --username"))?;
            store
                .set_roles(&u, &roles)
                .await
                .map_err(|_| CliError::usage("failed to set roles"))?;
            println!("roles updated for {u}");
        }
        "memberships-set" => {
            let u = username.ok_or_else(|| CliError::usage("missing --username"))?;
            store
                .set_memberships(&u, &memberships)
                .await
                .map_err(|_| CliError::usage("failed to set tenant memberships"))?;
            println!("tenant memberships updated for {u}");
        }
        "totp-enroll" => {
            let u = username.ok_or_else(|| CliError::usage("missing --username"))?;
            let (secret, codes) = store
                .enroll_totp(&u, recovery_count)
                .await
                .map_err(|_| CliError::usage("failed to enroll TOTP"))?;
            println!("TOTP secret (hex): {secret}");
            println!("TOTP secret (base32): {}", hex_to_base32(&secret)?);
            println!(
                "otpauth URI: otpauth://totp/Velran:{}?secret={}&issuer=Velran",
                u,
                hex_to_base32(&secret)?
            );
            println!("Recovery codes (store once, they are not shown again):");
            for c in codes {
                println!("{c}");
            }
        }
        "totp-disable" => {
            let u = username.ok_or_else(|| CliError::usage("missing --username"))?;
            store
                .disable_totp(&u)
                .await
                .map_err(|_| CliError::usage("failed to disable TOTP"))?;
            println!("TOTP disabled for {u}");
        }
        _ => return Err(CliError::usage(format!("unknown auth action `{action}`"))),
    }
    Ok(())
}

fn hex_to_base32(hex: &str) -> Result<String, CliError> {
    if hex.len() % 2 != 0 {
        return Err(CliError::usage("invalid hex secret"));
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for c in hex.as_bytes().chunks_exact(2) {
        let s = std::str::from_utf8(c)?;
        bytes.push(u8::from_str_radix(s, 16)?);
    }
    const A: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::new();
    let mut buffer = 0u32;
    let mut bits = 0u8;
    for b in bytes {
        buffer = (buffer << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(A[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(A[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    Ok(out)
}

fn read_password_file(path: &std::path::Path) -> Result<String, CliError> {
    let meta = std::fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(CliError::usage(
            "password file must be a regular non-symlink file",
        ));
    }
    if meta.len() > 4096 {
        return Err(CliError::usage("password file is too large"));
    }
    let v = std::fs::read_to_string(path)?;
    let v = v.trim_end_matches(&['\r', '\n'][..]).to_string();
    if v.contains('\n') || v.contains('\r') {
        return Err(CliError::usage("password file must contain one line"));
    }
    if v.len() < 12 || v.len() > 1024 {
        return Err(CliError::usage("password must be 12..1024 bytes"));
    }
    Ok(v)
}

fn print_usage_and_fail<T>() -> Result<T, CliError> {
    Err(CliError::usage(
        "Usage:\n  velran-cli check <app.vrn>\n  velran-cli migrate ...\n  velran-cli auth init --db-url-file <path>\n  velran-cli auth user-add --db-url-file <path> --username <name> --password-file <path> [--role Role] [--tenant Tenant]\n  velran-cli auth password-set|disable|enable|roles-set|memberships-set|totp-enroll|totp-disable ...",
    ))
}
