<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# Safe file-upload state machine

Velran treats file upload as a platform-owned state transition rather than as a trusted filename or MIME string supplied by the client.

Raw files stay private:

```vrn
#[action]
fn ingest(ctx: ActionContext, file: Upload) -> Result<Json, PageError> {
    return Ok(json(true));
}

route ingest POST "/imports"
    upload file<Upload> to "imports"
    auth user
    => ingest;
```

A browser-visible image requires an explicit publish intent:

```vrn
#[action]
fn saveHero(ctx: ActionContext, hero: Image) -> Result<Json, PageError> {
    return Ok(json(hero));
}

route saveHero POST "/admin/hero"
    upload hero<Image> to "media" publish
    auth user
    => saveHero;
```

The runtime lifecycle for a published image is:

```text
multipart bytes
    -> staged private file
    -> inspected image
    -> verified Image value
    -> handler success
    -> atomic publish into the declared media directory
```

The staging file is not addressable through the media endpoint. The platform derives the final storage name randomly; the client filename never becomes a filesystem path. The client `Content-Type` is metadata only and is not used to decide whether a file is an image.

For `Image`, the platform reads the staged bytes and accepts only supported magic-byte formats (currently PNG and JPEG), validates dimensions, applies the configured pixel ceiling, and constructs the typed `Image` value from the inspected bytes. Invalid images are deleted before they can be published.

If handler execution times out or returns an application error, the staged image is discarded. The public media endpoint only serves destinations declared by an `upload ...<Image> ... publish` route and re-inspects the stored bytes before serving them.

Raw `Upload` values cannot use `publish`. Their `filename` and `contentType` fields remain untrusted compiler values and therefore cannot cross protected mutation boundaries without explicit validation/refinement.

Compiler diagnostics:

- `SEC-FILE-001`: `Image` upload omitted the explicit `publish` transition.
- `SEC-FILE-002`: raw `Upload` attempted to use `publish`.

Security properties:

- bounded multipart body and per-file size;
- CSRF checked before the file field is accepted;
- generated storage filename;
- AppFs path confinement and no-follow filesystem operations;
- client filename is untrusted and cannot select storage paths;
- client MIME is untrusted and cannot establish image type;
- magic-byte image inspection before publication;
- image dimension/pixel limits;
- raw uploads remain private;
- public publication is explicit in source code;
- failed handler execution never publishes a staged image.

One exceptional-state issue remains intentionally separate: if the handler commits a database side effect successfully and the subsequent filesystem publish operation fails, the platform cannot currently roll both resources back atomically. That is part of the planned transaction/`CommitUnknown` semantics work and is not hidden by this feature.
