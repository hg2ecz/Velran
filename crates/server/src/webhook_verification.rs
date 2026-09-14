use crate::WebSecurityCliConfig;
use crate::http_io::{HttpRequest, Response};
use data::RedisStore;
use language_core::{Program, Route, RouteAuth, Webhook};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_SECRET_BYTES: usize = 64 * 1024;

type HmacSha256 = [u8; 32];

pub(super) async fn verify(
    program: &Program,
    route: &Route,
    request: &HttpRequest,
    redis: Option<&RedisStore>,
    domain_namespace: &str,
    web: &WebSecurityCliConfig,
    json_api: bool,
) -> Result<(), Response> {
    let RouteAuth::Webhook(name) = &route.auth else {
        return Ok(());
    };
    let webhook = program.webhook(name).ok_or_else(|| unavailable(json_api))?;
    let timestamp_raw = request
        .header(&webhook.timestamp_header)
        .ok_or_else(|| rejected(json_api, "webhook_timestamp_required"))?;
    let signature_raw = request
        .header(&webhook.signature_header)
        .ok_or_else(|| rejected(json_api, "webhook_signature_required"))?;
    let timestamp = timestamp_raw
        .parse::<u64>()
        .map_err(|_| rejected(json_api, "invalid_webhook_timestamp"))?;
    validate_timestamp(timestamp, webhook.replay_window_secs, json_api)?;
    let supplied = decode_hex_32(signature_raw)
        .ok_or_else(|| rejected(json_api, "invalid_webhook_signature"))?;
    let root = web
        .webhook_secrets_dir
        .as_deref()
        .ok_or_else(|| unavailable(json_api))?;
    let mut secret = read_secret(root, &webhook.secret_name).map_err(|_| unavailable(json_api))?;
    let expected = hmac_sha256(&secret, timestamp_raw.as_bytes(), &request.body);
    secret.fill(0);
    if !constant_time_eq(&expected, &supplied) {
        return Err(rejected(json_api, "invalid_webhook_signature"));
    }
    claim_replay(
        redis,
        domain_namespace,
        webhook,
        timestamp_raw,
        signature_raw,
        json_api,
    )
    .await
}

async fn claim_replay(
    redis: Option<&RedisStore>,
    domain: &str,
    webhook: &Webhook,
    timestamp: &str,
    signature: &str,
    json_api: bool,
) -> Result<(), Response> {
    let Some(redis) = redis else {
        return Err(unavailable(json_api));
    };
    let mut hash = Sha256::new();
    hash.update(domain.as_bytes());
    hash.update([0]);
    hash.update(webhook.name.as_bytes());
    hash.update([0]);
    hash.update(timestamp.as_bytes());
    hash.update([0]);
    hash.update(signature.as_bytes());
    let key = format!("webhook-replay:{:x}", hash.finalize());
    match redis
        .set_if_absent(&key, b"1", webhook.replay_window_secs)
        .await
    {
        Ok(true) => Ok(()),
        Ok(false) => Err(rejected(json_api, "webhook_replay_rejected")),
        Err(_) => Err(unavailable(json_api)),
    }
}

fn validate_timestamp(timestamp: u64, window: u64, json_api: bool) -> Result<(), Response> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| unavailable(json_api))?
        .as_secs();
    if now.abs_diff(timestamp) > window {
        return Err(rejected(
            json_api,
            "webhook_timestamp_outside_replay_window",
        ));
    }
    Ok(())
}

fn read_secret(root: &Path, name: &str) -> Result<Vec<u8>, ()> {
    if name.is_empty()
        || name.len() > 128
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(());
    }
    let path = root.join(name);
    let meta = std::fs::symlink_metadata(&path).map_err(|_| ())?;
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() as usize > MAX_SECRET_BYTES {
        return Err(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.mode() & 0o077 != 0 {
            return Err(());
        }
    }
    let mut opts = OpenOptions::new();
    opts.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let file: File = opts.open(path).map_err(|_| ())?;
    let mut value = Vec::with_capacity(meta.len() as usize);
    file.take((MAX_SECRET_BYTES + 1) as u64)
        .read_to_end(&mut value)
        .map_err(|_| ())?;
    if value.len() > MAX_SECRET_BYTES {
        value.fill(0);
        return Err(());
    }
    while matches!(value.last(), Some(b'\n' | b'\r')) {
        value.pop();
    }
    if value.is_empty() {
        return Err(());
    }
    Ok(value)
}

fn hmac_sha256(secret: &[u8], timestamp: &[u8], body: &[u8]) -> HmacSha256 {
    const BLOCK: usize = 64;
    let mut key = [0u8; BLOCK];
    if secret.len() > BLOCK {
        key[..32].copy_from_slice(&Sha256::digest(secret));
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let mut inner_pad = [0x36u8; BLOCK];
    let mut outer_pad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        inner_pad[i] ^= key[i];
        outer_pad[i] ^= key[i];
    }
    key.fill(0);
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(timestamp);
    inner.update(b".");
    inner.update(body);
    let inner_hash = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_hash);
    outer.finalize().into()
}

fn decode_hex_32(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    let bytes = value.as_bytes();
    for i in 0..32 {
        out[i] = (hex(bytes[i * 2])? << 4) | hex(bytes[i * 2 + 1])?;
    }
    Some(out)
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn constant_time_eq(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right.iter())
        .fold(0u8, |diff, (a, b)| diff | (a ^ b))
        == 0
}

fn rejected(json_api: bool, code: &str) -> Response {
    error(
        json_api,
        401,
        "Unauthorized",
        code,
        b"webhook verification failed\n",
    )
}

fn unavailable(json_api: bool) -> Response {
    error(
        json_api,
        503,
        "Service Unavailable",
        "webhook_verification_unavailable",
        b"webhook verification unavailable\n",
    )
}

fn error(json_api: bool, status: u16, reason: &'static str, code: &str, text: &[u8]) -> Response {
    if json_api {
        Response::new(
            status,
            reason,
            "application/json; charset=utf-8",
            format!("{{\"error\":\"{code}\"}}\n").as_bytes(),
        )
    } else {
        Response::text(status, reason, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_matches_known_timestamp_dot_body_vector() {
        let secret = [0x0bu8; 20];
        let actual = hmac_sha256(&secret, b"Hi There", b"");
        assert_eq!(
            actual,
            decode_hex_32("cea5909e8144cfc6203f8a0ac682729272a84acc9102fb7e3c268a8aa226d11d")
                .unwrap()
        );
    }

    #[test]
    fn hex_signature_is_strict() {
        assert!(decode_hex_32(&"ab".repeat(32)).is_some());
        assert!(decode_hex_32("abcd").is_none());
        assert!(decode_hex_32(&"gg".repeat(32)).is_none());
    }

    #[test]
    fn comparison_detects_difference() {
        let a = [7u8; 32];
        let mut b = a;
        assert!(constant_time_eq(&a, &b));
        b[31] ^= 1;
        assert!(!constant_time_eq(&a, &b));
    }
}
