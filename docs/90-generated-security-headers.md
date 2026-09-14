<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Generated CSP and security headers

Velran keeps browser security headers platform-owned. Application code cannot emit a weaker CSP or opt into inline script/style execution.

The HTTP response boundary now generates a least-privilege CSP for each response. HTML documents are inspected only for compiler-controlled static markup capabilities; dynamic HTML interpolation remains escaped by the structured HTML compiler/runtime boundary.

For HTML responses:

- `default-src 'none'`
- `script-src 'none'`
- `connect-src 'none'`
- `frame-src 'none'`
- `worker-src 'none'`
- `object-src 'none'`
- `base-uri 'none'`
- `frame-ancestors 'none'`
- `style-src 'self'` only when the document contains a stylesheet link, otherwise `'none'`
- `font-src 'self'` only when stylesheet loading is enabled, otherwise `'none'`
- `img-src 'self'` only when the document contains an image, otherwise `'none'`
- `media-src 'self'` only when the document contains audio/video/source markup, otherwise `'none'`
- `form-action 'self'` only when the document contains a form, otherwise `'none'`

Notably, the previous global `img-src ... data:` and `connect-src 'self'` allowances are removed. Server-side named outbound integrations do not grant browser-side network authority.

Non-HTML responses receive a small deny-by-default CSP suitable for browser navigation defense in depth.

Existing platform-owned headers remain in force: HSTS on effective TLS, `X-Content-Type-Options`, `Referrer-Policy`, COOP, CORP, `X-Frame-Options`, `Permissions-Policy`, and secure cache defaults.
