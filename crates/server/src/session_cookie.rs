use language_core::ServerConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SameSite {
    Lax,
    None,
}

impl SameSite {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Lax => "Lax",
            Self::None => "None",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SessionCookiePolicy {
    secure: bool,
    same_site: SameSite,
}

impl SessionCookiePolicy {
    fn from_config(config: &ServerConfig, cors_credentials: bool) -> Self {
        Self {
            secure: !config.insecure_dev_cookies,
            same_site: if cors_credentials {
                SameSite::None
            } else {
                SameSite::Lax
            },
        }
    }
}

pub(super) fn name(config: &ServerConfig) -> &'static str {
    if config.insecure_dev_cookies {
        "velran_session"
    } else {
        "__Host-velran_session"
    }
}

pub(super) fn render(config: &ServerConfig, id: &str, cors_credentials: bool) -> String {
    let policy = SessionCookiePolicy::from_config(config, cors_credentials);
    let secure = if policy.secure { "; Secure" } else { "" };
    format!(
        "{}={}; Path=/; HttpOnly; SameSite={}{}; Max-Age={}",
        name(config),
        id,
        policy.same_site.as_str(),
        secure,
        config.session_ttl_secs
    )
}

pub(super) fn parse<'a>(header: &'a str, wanted: &str) -> Option<&'a str> {
    let mut found = None;
    for pair in header.split(';') {
        let (name, value) = pair.trim().split_once('=')?;
        if name == wanted {
            if found.is_some() || value.is_empty() {
                return None;
            }
            found = Some(value);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_cookie_is_host_only_http_only_secure_and_lax_by_default() {
        let config = ServerConfig::default();
        let cookie = render(&config, "abc", false);
        assert!(cookie.starts_with("__Host-velran_session=abc; Path=/;"));
        assert!(cookie.contains("; HttpOnly;"));
        assert!(cookie.contains("; SameSite=Lax; Secure;"));
        assert!(!cookie.contains("Domain="));
    }

    #[test]
    fn credentialed_cross_origin_cookie_uses_same_site_none_but_remains_secure() {
        let config = ServerConfig::default();
        let cookie = render(&config, "abc", true);
        assert!(cookie.contains("; SameSite=None; Secure;"));
    }

    #[test]
    fn insecure_dev_mode_drops_host_prefix_and_secure_flag() {
        let mut config = ServerConfig::default();
        config.insecure_dev_cookies = true;
        let cookie = render(&config, "abc", false);
        assert!(cookie.starts_with("velran_session=abc;"));
        assert!(!cookie.contains("; Secure"));
    }

    #[test]
    fn duplicate_cookie_name_is_rejected() {
        assert_eq!(
            parse(
                "a=1; velran_session=one; velran_session=two",
                "velran_session"
            ),
            None
        );
    }
}
