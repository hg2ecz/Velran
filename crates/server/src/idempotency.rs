use crate::http_io::{HttpRequest, Response};
use data::RedisStore;
use language_core::Route;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const IDEMPOTENCY_TTL_SECS: u64 = 24 * 60 * 60;

pub(super) enum BeginResult {
    Disabled,
    Claimed(IdempotencyClaim),
    Replay(Response),
}

#[derive(Debug)]
pub(super) struct IdempotencyClaim {
    storage_key: String,
    fingerprint: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum StoredState {
    Pending {
        fingerprint: String,
    },
    Complete {
        fingerprint: String,
        response: StoredResponse,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredResponse {
    status: u16,
    body: Vec<u8>,
    headers: Vec<(String, String)>,
    content_type: String,
}

pub(super) async fn begin(
    redis: Option<&RedisStore>,
    request: &HttpRequest,
    route: &Route,
    principal: Option<&str>,
    domain_namespace: &str,
    json_api: bool,
) -> Result<BeginResult, Response> {
    if !route.idempotent {
        return Ok(BeginResult::Disabled);
    }
    let Some(principal) = principal else {
        return Err(error(
            json_api,
            401,
            "Unauthorized",
            "authentication_required",
            b"authentication required\n",
        ));
    };
    let Some(raw) = request.header("idempotency-key") else {
        return Err(error(
            json_api,
            400,
            "Bad Request",
            "idempotency_key_required",
            b"Idempotency-Key header required\n",
        ));
    };
    if !valid_key(raw) {
        return Err(error(
            json_api,
            400,
            "Bad Request",
            "invalid_idempotency_key",
            b"invalid Idempotency-Key\n",
        ));
    }
    let Some(redis) = redis else {
        return Err(error(
            json_api,
            503,
            "Service Unavailable",
            "idempotency_unavailable",
            b"idempotency unavailable\n",
        ));
    };
    let storage_key = storage_key(domain_namespace, &route.name, principal, raw);
    let fingerprint = request_fingerprint(request);
    let pending = StoredState::Pending {
        fingerprint: fingerprint.clone(),
    };
    let encoded = serde_json::to_vec(&pending).map_err(|_| unavailable(json_api))?;
    match redis
        .set_if_absent(&storage_key, &encoded, IDEMPOTENCY_TTL_SECS)
        .await
    {
        Ok(true) => Ok(BeginResult::Claimed(IdempotencyClaim {
            storage_key,
            fingerprint,
        })),
        Ok(false) => load_existing(redis, &storage_key, &fingerprint, json_api).await,
        Err(_) => Err(unavailable(json_api)),
    }
}

pub(super) async fn complete(
    redis: Option<&RedisStore>,
    claim: IdempotencyClaim,
    response: &Response,
) {
    if response.status >= 500 {
        return;
    }
    let Some(redis) = redis else {
        return;
    };
    let state = StoredState::Complete {
        fingerprint: claim.fingerprint,
        response: StoredResponse::from_response(response),
    };
    let Ok(encoded) = serde_json::to_vec(&state) else {
        return;
    };
    let _ = redis
        .set(&claim.storage_key, &encoded, Some(IDEMPOTENCY_TTL_SECS))
        .await;
}

async fn load_existing(
    redis: &RedisStore,
    key: &str,
    fingerprint: &str,
    json_api: bool,
) -> Result<BeginResult, Response> {
    let bytes = redis
        .get(key)
        .await
        .map_err(|_| unavailable(json_api))?
        .ok_or_else(|| unavailable(json_api))?;
    let state: StoredState = serde_json::from_slice(&bytes).map_err(|_| unavailable(json_api))?;
    match state {
        StoredState::Pending {
            fingerprint: stored,
        } => {
            if stored != fingerprint {
                Err(key_reused(json_api))
            } else {
                Err(error(
                    json_api,
                    409,
                    "Conflict",
                    "request_in_progress",
                    b"idempotent request is still in progress\n",
                ))
            }
        }
        StoredState::Complete {
            fingerprint: stored,
            response,
        } => {
            if stored != fingerprint {
                Err(key_reused(json_api))
            } else {
                response
                    .into_response()
                    .map(BeginResult::Replay)
                    .map_err(|_| unavailable(json_api))
            }
        }
    }
}

impl StoredResponse {
    fn from_response(response: &Response) -> Self {
        Self {
            status: response.status,
            body: response.body.clone(),
            headers: response.stored_headers(),
            content_type: response.content_type().to_string(),
        }
    }

    fn into_response(self) -> Result<Response, ()> {
        let reason = reason_for_status(self.status).ok_or(())?;
        let content_type = content_type(&self.content_type).ok_or(())?;
        let mut response = Response::new(self.status, reason, content_type, &self.body);
        if !response.restore_headers(self.headers) {
            return Err(());
        }
        Ok(response)
    }
}

fn request_fingerprint(request: &HttpRequest) -> String {
    let mut hash = Sha256::new();
    hash.update(request.method.as_bytes());
    hash.update([0]);
    hash.update(request.target.as_bytes());
    hash.update([0]);
    if let Some(content_type) = request.header("content-type") {
        hash.update(content_type.as_bytes());
    }
    hash.update([0]);
    hash.update(&request.body);
    format!("{:x}", hash.finalize())
}

fn valid_key(value: &str) -> bool {
    (16..=128).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
}

fn storage_key(domain: &str, route: &str, principal: &str, raw: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(domain.as_bytes());
    hash.update([0]);
    hash.update(route.as_bytes());
    hash.update([0]);
    hash.update(principal.as_bytes());
    hash.update([0]);
    hash.update(raw.as_bytes());
    format!("idempotency:{:x}", hash.finalize())
}

fn key_reused(json_api: bool) -> Response {
    error(
        json_api,
        409,
        "Conflict",
        "idempotency_key_reused",
        b"Idempotency-Key was already used for another request\n",
    )
}

fn unavailable(json_api: bool) -> Response {
    error(
        json_api,
        503,
        "Service Unavailable",
        "idempotency_unavailable",
        b"idempotency unavailable\n",
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

fn reason_for_status(status: u16) -> Option<&'static str> {
    match status {
        200 => Some("OK"),
        201 => Some("Created"),
        202 => Some("Accepted"),
        204 => Some("No Content"),
        302 => Some("Found"),
        303 => Some("See Other"),
        307 => Some("Temporary Redirect"),
        308 => Some("Permanent Redirect"),
        400 => Some("Bad Request"),
        401 => Some("Unauthorized"),
        403 => Some("Forbidden"),
        404 => Some("Not Found"),
        405 => Some("Method Not Allowed"),
        406 => Some("Not Acceptable"),
        409 => Some("Conflict"),
        415 => Some("Unsupported Media Type"),
        429 => Some("Too Many Requests"),
        _ => None,
    }
}

fn content_type(value: &str) -> Option<&'static str> {
    match value {
        "application/json; charset=utf-8" => Some("application/json; charset=utf-8"),
        "text/html; charset=utf-8" => Some("text/html; charset=utf-8"),
        "text/plain; charset=utf-8" => Some("text/plain; charset=utf-8"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_validation_is_bounded_and_header_safe() {
        assert!(valid_key("0123456789abcdef"));
        assert!(valid_key("checkout_2026-09-07:abc.def"));
        assert!(!valid_key("short"));
        assert!(!valid_key("bad key with spaces"));
        assert!(!valid_key("line\nbreak-0123456789"));
    }

    #[test]
    fn storage_key_does_not_embed_secret_or_principal_text() {
        let key = storage_key("shop", "charge", "alice@example.com", "0123456789abcdef");
        assert!(key.starts_with("idempotency:"));
        assert!(!key.contains("alice"));
        assert!(!key.contains("0123456789abcdef"));
    }

    #[test]
    fn fingerprint_changes_with_payload() {
        let request = |body: &[u8]| HttpRequest {
            method: "POST".into(),
            target: "/charge".into(),
            headers: vec![("content-type".into(), "application/json".into())],
            body: body.to_vec(),
        };
        assert_ne!(
            request_fingerprint(&request(b"a")),
            request_fingerprint(&request(b"b"))
        );
    }
}
