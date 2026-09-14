use crate::response_headers::HeaderName;
use auth::{
    AuthAbuseStage, AuthError, SessionBackend, SessionSnapshot, authenticate_ldap,
    verify_totp_redis,
};
use language_core::ServerConfig;
use runtime::decode_urlencoded_limited;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::auth_observability::{
    audit_auth_activity, audit_auth_activity_action, observe_auth_security,
};
use super::auth_setup::canonical_username;
use super::http_io::{HttpRequest, Response};
use super::{AuthRuntime, WebSecurityCliConfig};

fn authentication_unavailable() -> Response {
    Response::text(503, "Service Unavailable", b"authentication unavailable\n")
}

async fn record_auth_failure(
    auth: &AuthRuntime,
    stage: AuthAbuseStage,
    source: &str,
    principal: &str,
) -> Response {
    match auth.limiter.record_failure(stage, source, principal).await {
        Ok(()) => Response::text(401, "Unauthorized", b"invalid credentials\n"),
        Err(AuthError::RateLimited) => Response::text(
            429,
            "Too Many Requests",
            b"too many authentication attempts\n",
        ),
        Err(_) => authentication_unavailable(),
    }
}

pub(super) async fn auth_login(
    request: &HttpRequest,
    config: &ServerConfig,
    sessions: &SessionBackend,
    session: &SessionSnapshot,
    auth: &AuthRuntime,
    request_id: &str,
    peer_key: &str,
    web: &WebSecurityCliConfig,
) -> Response {
    if request.method == "GET" {
        let body = format!(
            "<!doctype html><html><body><h1>Login</h1><form method=\"post\" action=\"/__velran/auth/login\"><input type=\"hidden\" name=\"_csrf\" value=\"{}\"><label>User <input name=\"username\" autocomplete=\"username\"></label><label>Password <input type=\"password\" name=\"password\" autocomplete=\"current-password\"></label><label>Second factor <input name=\"totp\" autocomplete=\"one-time-code\"></label><button>Login</button></form></body></html>",
            session.csrf_token
        );
        return Response::new(200, "OK", "text/html; charset=utf-8", body.as_bytes());
    }
    if request.method != "POST" {
        return Response::text(405, "Method Not Allowed", b"method not allowed\n");
    }
    if auth.ldap.is_none() && auth.local.is_none() {
        return authentication_unavailable();
    }
    if !request.header("content-type").is_some_and(|v| {
        v.split(';')
            .next()
            .unwrap_or("")
            .trim()
            .eq_ignore_ascii_case("application/x-www-form-urlencoded")
    }) {
        return Response::text(415, "Unsupported Media Type", b"expected form body\n");
    }
    let pairs = match decode_urlencoded_limited(&request.body, 8, 4096) {
        Ok(v) => v,
        Err(_) => return Response::text(400, "Bad Request", b"bad form body\n"),
    };
    let mut map = HashMap::new();
    for (k, v) in pairs {
        if map.insert(k, v).is_some() {
            return Response::text(400, "Bad Request", b"duplicate form field\n");
        }
    }
    if map.len() != 4 {
        return Response::text(400, "Bad Request", b"invalid login form\n");
    }
    let (Some(csrf), Some(raw_username), Some(password), Some(code)) = (
        map.remove("_csrf"),
        map.remove("username"),
        map.remove("password"),
        map.remove("totp"),
    ) else {
        return Response::text(400, "Bad Request", b"invalid login form\n");
    };
    if !matches!(sessions.verify_csrf(&session.id, &csrf).await, Ok(true)) {
        return Response::text(403, "Forbidden", b"CSRF validation failed\n");
    }
    let username = match canonical_username(&raw_username) {
        Some(v) => v,
        None => return Response::text(400, "Bad Request", b"invalid login form\n"),
    };
    if password.len() > 4096 || code.len() > 64 {
        return Response::text(400, "Bad Request", b"invalid login form\n");
    }
    match auth
        .limiter
        .check(AuthAbuseStage::Password, peer_key, &username)
        .await
    {
        Ok(()) => {}
        Err(AuthError::RateLimited) => {
            audit_auth_activity(request_id, &username, "rate_limited", peer_key);
            observe_auth_security("auth", "rate_limited", request_id, peer_key, &username);
            return Response::text(
                429,
                "Too Many Requests",
                b"too many authentication attempts\n",
            );
        }
        Err(_) => {
            return authentication_unavailable();
        }
    }
    let (canonical_principal, roles, memberships, secret, auth_generation, local_backend) =
        if let Some(local) = auth.local.as_ref() {
            match local.authenticate(&username, &password).await {
                Ok(user) => (
                    user.username,
                    user.roles,
                    user.memberships,
                    user.totp_secret,
                    user.auth_generation,
                    true,
                ),
                Err(AuthError::StoreUnavailable) => {
                    return authentication_unavailable();
                }
                Err(_) => {
                    audit_auth_activity(request_id, &username, "invalid_credentials", peer_key);
                    observe_auth_security(
                        "auth",
                        "invalid_credentials",
                        request_id,
                        peer_key,
                        &username,
                    );
                    return record_auth_failure(
                        auth,
                        AuthAbuseStage::Password,
                        peer_key,
                        &username,
                    )
                    .await;
                }
            }
        } else {
            let Some(ldap) = auth.ldap.as_ref() else {
                return authentication_unavailable();
            };
            if authenticate_ldap(ldap, &username, &password).await.is_err() {
                audit_auth_activity(request_id, &username, "invalid_credentials", peer_key);
                observe_auth_security(
                    "auth",
                    "invalid_credentials",
                    request_id,
                    peer_key,
                    &username,
                );
                return record_auth_failure(auth, AuthAbuseStage::Password, peer_key, &username)
                    .await;
            }
            let roles = auth
                .roles
                .get(&username)
                .cloned()
                .unwrap_or_else(|| vec!["User".into()]);
            let memberships = auth.memberships.get(&username).cloned().unwrap_or_default();
            let auth_generation = auth.mapped_claims_generation(&username);
            (
                username.clone(),
                roles,
                memberships,
                auth.totp_secrets.get(&username).cloned(),
                auth_generation,
                false,
            )
        };
    let mfa = if let Some(secret) = secret.as_deref() {
        match auth
            .limiter
            .check(AuthAbuseStage::Mfa, peer_key, &canonical_principal)
            .await
        {
            Ok(()) => {}
            Err(AuthError::RateLimited) => {
                audit_auth_activity(
                    request_id,
                    &canonical_principal,
                    "mfa_rate_limited",
                    peer_key,
                );
                observe_auth_security(
                    "mfa",
                    "rate_limited",
                    request_id,
                    peer_key,
                    &canonical_principal,
                );
                return Response::text(
                    429,
                    "Too Many Requests",
                    b"too many authentication attempts\n",
                );
            }
            Err(_) => {
                return authentication_unavailable();
            }
        }
        let unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|v| v.as_secs())
            .unwrap_or(0);
        let totp_ok = code.len() == 6
            && code.bytes().all(|b| b.is_ascii_digit())
            && if let Some(redis) = &auth.redis {
                verify_totp_redis(redis, &canonical_principal, secret, unix, &code)
                    .await
                    .is_ok()
            } else {
                auth.local_totp
                    .verify(&canonical_principal, secret, unix, &code)
                    .is_ok()
            };
        let recovery_ok = if !totp_ok && local_backend {
            let Some(local) = auth.local.as_ref() else {
                return authentication_unavailable();
            };
            match local
                .consume_recovery_code(&canonical_principal, &code)
                .await
            {
                Ok(v) => v,
                Err(_) => {
                    return authentication_unavailable();
                }
            }
        } else {
            false
        };
        if !totp_ok && !recovery_ok {
            audit_auth_activity(
                request_id,
                &canonical_principal,
                "invalid_second_factor",
                peer_key,
            );
            observe_auth_security(
                "mfa",
                "invalid_second_factor",
                request_id,
                peer_key,
                &canonical_principal,
            );
            return record_auth_failure(auth, AuthAbuseStage::Mfa, peer_key, &canonical_principal)
                .await;
        }
        let _ = auth
            .limiter
            .clear_pair(AuthAbuseStage::Mfa, peer_key, &canonical_principal)
            .await;
        true
    } else if auth.require_totp {
        audit_auth_activity(
            request_id,
            &canonical_principal,
            "second_factor_required",
            peer_key,
        );
        observe_auth_security(
            "mfa",
            "required",
            request_id,
            peer_key,
            &canonical_principal,
        );
        return Response::text(401, "Unauthorized", b"invalid credentials\n");
    } else {
        false
    };
    let _ = auth
        .limiter
        .clear_pair(AuthAbuseStage::Password, peer_key, &canonical_principal)
        .await;
    let rotated = match sessions
        .rotate_authenticated(
            &session.id,
            canonical_principal.clone(),
            mfa,
            roles,
            memberships,
            auth_generation,
        )
        .await
    {
        Ok(v) => v,
        Err(_) => {
            return authentication_unavailable();
        }
    };
    let mut response = Response::redirect(303, "See Other", "/");
    response.push_header(
        HeaderName::SetCookie,
        crate::session_cookie::render(config, &rotated.id, web.cors_allow_credentials),
    );
    audit_auth_activity(request_id, &canonical_principal, "success", peer_key);
    response
}

pub(super) async fn auth_logout(
    request: &HttpRequest,
    config: &ServerConfig,
    sessions: &SessionBackend,
    session: &SessionSnapshot,
    request_id: &str,
    peer_key: &str,
    web: &WebSecurityCliConfig,
) -> Response {
    if request.method != "POST" {
        return Response::text(405, "Method Not Allowed", b"method not allowed\n");
    };
    let pairs = match decode_urlencoded_limited(&request.body, 4, 4096) {
        Ok(v) => v,
        Err(_) => return Response::text(400, "Bad Request", b"bad form body\n"),
    };
    let csrf = pairs
        .iter()
        .filter(|(k, _)| k == "_csrf")
        .collect::<Vec<_>>();
    if csrf.len() != 1
        || !matches!(
            sessions.verify_csrf(&session.id, &csrf[0].1).await,
            Ok(true)
        )
    {
        return Response::text(403, "Forbidden", b"CSRF validation failed\n");
    };
    if sessions.invalidate(&session.id).await.is_err() {
        return authentication_unavailable();
    };
    let fresh = match sessions.create().await {
        Ok(v) => v,
        Err(_) => {
            return authentication_unavailable();
        }
    };
    let mut response = Response::redirect(303, "See Other", "/");
    response.push_header(
        HeaderName::SetCookie,
        crate::session_cookie::render(config, &fresh.id, web.cors_allow_credentials),
    );
    let actor = session.principal.as_deref().unwrap_or("anonymous");
    audit_auth_activity_action(request_id, actor, "logout", "success", peer_key);
    response
}

pub(super) fn session_cookie_name(config: &ServerConfig) -> &'static str {
    crate::session_cookie::name(config)
}

pub(super) fn parse_cookie<'a>(header: &'a str, wanted: &str) -> Option<&'a str> {
    crate::session_cookie::parse(header, wanted)
}
