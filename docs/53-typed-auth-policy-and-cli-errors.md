<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Typed authentication, policy, resource-profile, and CLI errors

R10 continues the server clean-code work by pushing typed errors to the remaining configuration and command-line leaf boundaries.

## Design rule

Leaf modules must preserve error categories and useful source errors. The process-level startup function may still aggregate heterogeneous crate errors behind `Box<dyn Error>` because it is the application boundary, not a reusable leaf API.

## Authentication setup

`auth_setup.rs` returns `AuthSetupError` instead of `Box<dyn Error>`. Distinct variants cover missing LDAP fields, LDAP validation, auth-file I/O, malformed TOTP/role lines, invalid or duplicate usernames, invalid hex secrets, short TOTP secrets, and invalid role names.

This makes startup failures testable without matching human-readable strings.

## Rate-policy configuration

`load_rate_policies` and `validate_route_rate_policies` return `RatePolicyConfigError`.

The type distinguishes file I/O, line-oriented syntax errors, missing fields, invalid numbers, invalid limits, unknown scopes/keys, unknown route policies, and public routes attempting to use user-scoped policies.

## Resource-profile configuration

`load_resource_profiles` and `audit_resource_profiles` return `ResourceProfileConfigError`.

Parsing errors retain the source file and line. Numeric parse and integer-conversion errors remain available through `Error::source()`. Runtime `ResourceProfileError` is preserved instead of flattened into a string.

## CLI parsing

`cli::parse_args()` now returns `CliParseError` rather than `Box<dyn Error>`.

The CLI error type is an application-facing sum type over server config, secret-file, host/static-prefix, reserved-path, numeric CLI value, policy/profile, compiler, TLS, I/O, address, and integer-conversion failures. Direct CLI validation failures remain an explicit `Invalid` variant.

Small numeric helpers return `CliValueError`, which distinguishes missing values, invalid numbers, and zero where a positive value is required.

## Error-module layout

Typed errors are split by responsibility rather than collected in a new god file:

- `server_errors.rs`: shared server-config/TLS boundary errors and re-exports;
- `server_errors/auth_setup.rs`: authentication bootstrap errors;
- `server_errors/policy_config.rs`: rate-policy and resource-profile errors;
- `server_errors/cli.rs`: CLI and reserved-endpoint errors.

The clean-structure check protects these boundaries and rejects regressions back to boxed errors in the leaf APIs.
