<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Secure data types and authorization proofs

Velran treats web security metadata as part of static type checking rather than as optional framework advice.

## External input is untrusted by default

Scalar parameters of page and action handlers cross an HTTP trust boundary. The compiler therefore tracks them as untrusted internally without requiring `Untrusted<T>` boilerplate in every handler signature. Safe operations such as parameterized queries and escaped HTML may consume untrusted values; dangerous boundaries must reject them unless a stronger typed abstraction exists.

## Model data classification

Model fields can be classified explicitly:

```vrn
model User {
    id: Uuid
    owner: String
    email: Sensitive<Email>
    passwordHash: Secret<String>
}
```

`Public` is the default classification. The ordering is `Public < Sensitive < Secret`, and derived scalar expressions conservatively keep the strongest classification of their inputs.

`Secret<T>` is intentionally strict. A secret cannot be serialized into a JSON response or rendered into HTML, including through a local alias. This is a compile-time error (`SEC-DATA-001` / `SEC-DATA-002`).

## Flow-sensitive authorization proof

A loaded model begins with unverified access. The existing object authorization statement is also a static type refinement:

```vrn
let profile = loadProfile(db, id)?;
authorize profile owner owner;
let email = profile.email;
return Ok(json(email));
```

After `authorize profile ...;`, the compiler treats `profile` as an authorized model for the remainder of that straight-line handler flow. Sensitive fields derived from it carry authorization evidence through local scalar aliases and expressions.

Without the proof, disclosure is rejected:

```vrn
let profile = loadProfile(db, id)?;
return Ok(json(profile.email)); // compile error SEC-A01-004
```

The same rule applies to HTML interpolation, markdown output, and image/alt expressions.

Authorization remains enforced at runtime by the existing `Authorize` statement. The compiler does not duplicate the runtime access-control decision; it tracks the fact that the check is guaranteed to execute before the disclosure boundary.

## Why the proof is implicit

Velran deliberately does not require application authors to write wrapper-heavy types such as `Authorized<Profile, Read>` in ordinary handler code. `authorize` refines the existing variable instead. This keeps the source ergonomic while preserving a proof in the compiler's static environment.

The compiler only refines unconditional, straight-line authorization statements. Authorization hidden inside conditional compute flow does not currently create a proof outside that flow.

## Current boundary

This layer protects sensitive response disclosure. The next proof-oriented layer should extend the same model to mutations, so security-sensitive write operations can require authorization evidence rather than relying on convention.
