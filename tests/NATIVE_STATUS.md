<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Example native status

This file is generated/reviewed for the native-only architecture. It distinguishes runnable native-core sources from security rejection fixtures and examples still waiting for DB/transaction/advanced-template native lowering.

## Native-ready source files

- `examples/auth/app.vrn`
- `examples/business-types/app.vrn`
- `examples/cache/app.vrn`
- `examples/domain-types/main.vrn`
- `examples/domain-validation-m42/main.vrn`
- `examples/fft4096/main.vrn`
- `examples/forms/app.vrn`
- `examples/function-permissions/main.vrn`
- `examples/hello/app.vrn`
- `examples/media/app.vrn`
- `examples/mfa-elevation/main.vrn`
- `examples/module-namespaces/catalog.vrn`
- `examples/module-namespaces/main.vrn`
- `examples/news-site/components.vrn`
- `examples/news-site/main.vrn`
- `examples/news-site/models.vrn`
- `examples/numeric-operators/main.vrn`
- `examples/outbound-native/app.vrn`
- `examples/prg-flash/app.vrn`
- `examples/rate-limit/app.vrn`
- `examples/regex/main.vrn`
- `examples/resource-profile/app.vrn`
- `examples/starter-project/actions.vrn`
- `examples/starter-project/main.vrn`
- `examples/starter-project/models.vrn`
- `examples/starter-project/pages.vrn`
- `examples/static-assets/app.vrn`
- `examples/string-dict/main.vrn`
- `examples/string-list/main.vrn`
- `examples/string-operations/main.vrn`
- `examples/upload/app.vrn`

## Compile-gated language compatibility

`examples/markdown/app.vrn` is used by the M32 safe-Markdown verification as a real `velran-cli check` language-compatibility gate. Its presence in the host/lowering-pending list below does not mean the frontend language is allowed to skip it; it only records that the complete example may still depend on native host/lowering coverage outside the pure/CommonMark language surface.

The CommonMark source itself therefore must continue to compile as ordinary Velran and may not receive a Markdown-specific compiler exception.

## Native host/lowering pending source files

- `examples/a-z-products/main.vrn`
- `examples/business-audit/main.vrn`
- `examples/canonical-url/main.vrn`
- `examples/components/app.vrn`
- `examples/critical-operations/main.vrn`
- `examples/crud/app.vrn`
- `examples/database/app.vrn`
- `examples/domain-objects/main.vrn`
- `examples/domain-validation/main.vrn`
- `examples/enums/main.vrn`
- `examples/json-api/app.vrn`
- `examples/markdown/app.vrn`
- `examples/markdown-sql-cache/main.vrn`
- `examples/module-namespaces/catalog/pages.vrn`
- `examples/module-namespaces/catalog/queries.vrn`
- `examples/news-site/actions.vrn`
- `examples/news-site/pages.vrn`
- `examples/news-site/queries.vrn`
- `examples/nominal-domain-types/main.vrn`
- `examples/object-authorization/app.vrn`
- `examples/optimistic-locking/main.vrn`
- `examples/starter-project/queries.vrn`
- `examples/typed-security-events/main.vrn`
- `examples/wiki/main.vrn`

## Negative/security fixtures

- `tests/fixtures/negative/cache-user-dependent-rejected.vrn`
- `tests/fixtures/negative/form-schema-bad-validation-rejected.vrn`
- `tests/fixtures/negative/image-direct-interpolation-rejected.vrn`
- `tests/fixtures/negative/json-form-mixed-rejected.vrn`
- `tests/fixtures/negative/json-upload-mixed-rejected.vrn`
- `tests/fixtures/negative/object-auth-public-rejected.vrn`
- `tests/fixtures/security/auth-mode-rejected.vrn`
- `tests/fixtures/security/list-direct-interpolation-rejected.vrn`
- `tests/fixtures/security/optional-without-check-rejected.vrn`
- `tests/fixtures/security/sql-bind-mismatch-rejected.vrn`
- `tests/fixtures/security/sql-injection-rejected.vrn`
- `tests/fixtures/security/unsafe-dynamic-href-rejected.vrn`
- `tests/fixtures/security/upload-form-binding-rejected.vrn`
- `tests/fixtures/security/upload-path-rejected.vrn`
- `tests/fixtures/security/url-helper-type-rejected.vrn`
- `tests/fixtures/security/validation-schema-rejected.vrn`
- `tests/fixtures/security/xss-attribute-rejected.vrn`
- `tests/fixtures/security/xss-script-rejected.vrn`
