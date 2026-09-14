<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# 1. Gyors kezdés

## 1. Készíts alkalmazáskönyvtárat

Ajánlott entrypoint: `main.vrn`.

```text
myapp/
├── main.vrn
├── pages.vrn
└── public/
```

`main.vrn`:

```velran
mod pages;
```

`pages.vrn`:

```velran
#[page]
fn home(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {
        <main><h1>Hello Velran</h1></main>
    });
}

route home GET "/" public => home;
```

A `home` rövid handlernév itt azért érvényes, mert a route és a page ugyanabban a `pages` modulban van. A `mod pages;` nem emeli be a `home` nevet a `main.vrn` scope-jába. Ha a route a `main.vrn`-ben lenne, explicit `pages::home` handlernév kellene.

## 2. Ellenőrizd a workspace-t

```bash
./verify.sh
```

Minimum fejlesztés közben:

```bash
cargo check --workspace
cargo test --workspace
```

## 3. Indítsd development módban

```bash
cargo run -p velran-server -- \
  --app /abszolut/path/myapp/main.vrn \
  --listen 127.0.0.1:8080 \
  --insecure-dev-cookies
```

```bash
curl -i http://127.0.0.1:8080/
```

Az `--insecure-dev-cookies` kizárólag local developmentre való.

## 4. Production: config az elsődleges

Másold és igazítsd a `config/server.toml.sample` fájlt, majd:

```bash
./velran-server \
  --config /usr/local/etc/velran/server.toml \
  --app /srv/myapp/main.vrn
```

A `--app` CLI override. Ha nem akarod CLI-ben megadni, tedd a TOML `[server]` szekciójába. A precedence: `defaults < config < CLI`.

Credentialet productionban fájlból adj át (`*_file`), ne shell argumentként és ne plaintext TOML mezőben.

Következő: [Projektstruktúra, modulok és keresőbarát URL-ek](21-modules-slugs-project-layout.md).
