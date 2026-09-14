use super::*;

#[cfg(test)]
mod m44_conflict_ux_tests {
    use super::*;

    #[test]
    fn html_conflict_is_human_readable_and_no_store() {
        let response = conflict_response(false);
        assert_eq!(response.status, 409);
        assert!(String::from_utf8_lossy(&response.body).contains("The data changed"));
    }

    #[test]
    fn json_conflict_keeps_stable_envelope() {
        let response = conflict_response(true);
        assert_eq!(response.status, 409);
        assert_eq!(
            String::from_utf8_lossy(&response.body),
            r#"{"error":"conflict"}"#
        );
    }
}

#[cfg(test)]
mod typed_server_boundary_error_tests {
    use super::*;

    #[test]
    fn public_host_validation_returns_typed_error() {
        let err = validate_public_host("evil.example/path").unwrap_err();
        assert!(matches!(&err, PublicHostError { .. }));
        assert!(err.to_string().contains("invalid public host"));
    }

    #[test]
    fn static_prefix_validation_returns_typed_error() {
        assert!(matches!(
            validate_static_prefix("assets"),
            Err(StaticPrefixError::Shape)
        ));
        assert!(matches!(
            validate_static_prefix("/../"),
            Err(StaticPrefixError::DotSegment)
        ));
    }

    #[test]
    fn secret_file_reports_empty_secret_separately() {
        let path = std::env::temp_dir().join(format!(
            "velran-empty-secret-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        fs::write(&path, "\n").unwrap();
        let err = read_secret_file(path.to_str().unwrap()).unwrap_err();
        assert!(matches!(err, SecretFileError::Empty { .. }));
        let _ = fs::remove_file(path);
    }
}
