<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Biztonságos adattípusok és autorizációs bizonyítékok

A Velran a webes biztonsági metaadatokat a statikus típusellenőrzés részének tekinti, nem opcionális framework-ajánlásnak.

## A külső input alapból untrusted

A page/action handler skalár paraméterei HTTP trust boundaryn érkeznek. A compiler ezért belsőleg untrustedként követi őket anélkül, hogy minden signature-ben `Untrusted<T>` boilerplate-et kellene írni. A biztonságos műveletek, például a paraméterezett query és az escaped HTML fogadhatnak ilyen értéket; veszélyes boundary csak erősebb típusos absztrakción keresztül engedheti tovább.

## Modell-adatklasszifikáció

A modellmezők explicit osztályozhatók:

```vrn
model User {
    id: Uuid
    owner: String
    email: Sensitive<Email>
    passwordHash: Secret<String>
}
```

A `Public` az alapértelmezett. A sorrend `Public < Sensitive < Secret`, és a származtatott skalár kifejezések konzervatívan megtartják a bemenetek legerősebb osztályozását.

A `Secret<T>` szándékosan szigorú: secret nem kerülhet JSON-válaszba vagy HTML-kimenetbe, köztes lokális alias esetén sem. Ez fordítási hiba (`SEC-DATA-001`, `SEC-DATA-002`).

## Flow-sensitive autorizációs bizonyíték

Egy betöltött modell hozzáférése kezdetben nem igazolt. A meglévő objektum-autorizációs statement egyben statikus típusfinomítás is:

```vrn
let profile = loadProfile(db, id)?;
authorize profile owner owner;
let email = profile.email;
return Ok(json(email));
```

Az `authorize profile ...;` után a compiler a `profile` változót autorizált modellként kezeli a handler további egyenes vezérlési útján. Az ebből származó `Sensitive<T>` értékek az autorizációs bizonyítékot lokális aliasokon és kifejezéseken keresztül is továbbviszik.

Bizonyíték nélkül a disclosure tiltott:

```vrn
let profile = loadProfile(db, id)?;
return Ok(json(profile.email)); // compile error SEC-A01-004
```

Ugyanez a szabály vonatkozik HTML interpolációra, markdown kimenetre, valamint image/alt kifejezésekre.

Az autorizációt runtime-ban továbbra is a meglévő `Authorize` statement hajtja végre. A compiler nem duplikálja a hozzáférési döntést, hanem azt bizonyítja, hogy az ellenőrzés a disclosure boundary előtt biztosan lefut.

## Miért implicit a proof?

A Velran szándékosan nem kényszeríti a fejlesztőt wrapper-zajos `Authorized<Profile, Read>` típusok kiírására a hétköznapi handler kódban. Az `authorize` ugyanazt a változót finomítja. Így a forrás egyszerű marad, miközben a compiler statikus környezetében megvan a bizonyíték.

A compiler jelenleg csak feltétel nélküli, egyenes vezérlési úton lévő autorizációt finomít proof-fá. Feltételes compute flow-ban elrejtett ellenőrzés nem hoz létre a blokkon kívül használható bizonyítékot.

## Jelenlegi határ

Ez a réteg az érzékeny response disclosure-t védi. A következő proof-alapú lépés ugyanennek a modellnek a kiterjesztése a mutációkra, hogy biztonságkritikus írási műveletek konvenció helyett autorizációs bizonyítékot követeljenek.
