<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 9. Upload és AppFs

## Route

```text
route uploadFile POST "/upload"
    upload file<Upload> to "uploads"
    public => uploadFile;
```

## Action

```text
#[action]
fn uploadFile(ctx: ActionContext, file: Upload)
    -> Result<Redirect, PageError> {
    let savedPath = file.path;
    let originalName = file.filename;
    let contentType = file.contentType;
    let byteCount = file.bytes;
    return Ok(redirect("/uploaded"));
}
```

Az `Upload` csak metadata; a teljes fájl nem kerül alkalmazás-memóriába. Native-only módban az Upload/Image handler ABI jelenleg fail-closed módon tiltott, amíg a typed host ABI elkészül.

## Server config

```bash
--data-root /srv/app/data
--fs-mode rwc
--max-upload-bytes 16777216
```

Uploadhoz `c` és `w` szükséges.

## Security

A kliens filename/MIME csak metadata. A runtime random storage key-t használ, staging fájlba streamel, majd atomikusan commitol. Linuxon az AppFs `openat2` confinementet használ (`BENEATH`, no symlink/magic-link/xdev).

Action/DB hiba esetén a runtime cleanupot kísérel meg.

## Typed image upload (M33)

For CMS/media use, prefer `Image` over generic `Upload`:

```velran
route uploadHero POST "/admin/hero" upload hero<Image> to "media" publish auth user => uploadHero;
```

The server validates PNG/JPEG bytes after streaming them into the confined AppFs location. Client MIME and filename are not trusted. Image routes require `rwc` AppFs mode. See [Biztonságos képek és media library](17-media-library.md).
