<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# Típusos credentialek és jelszó-primitívek

A Velran szándékosan magas szintű jelszóműveleteket ad raw kriptográfiai algoritmusok helyett.

```vrn
let hash = passwordHash(password);
let valid = passwordVerify(account.passwordHash, password);
```

A `passwordHash(Password)` validált, purpose-típusos jelszót fogad, és statikusan `Secret<PasswordHash>` adatot eredményez. A runtime Argon2id algoritmust, friss véletlen saltot és runtime-policy által birtokolt paramétereket használ. Az alkalmazáskód normál esetben nem választ algoritmust, saltot vagy cost paramétert.

A `passwordVerify(Secret<PasswordHash>, Password)` kizárólag a pontos password-hash purpose-t és validált purpose-típusos jelszót fogadja. Az eredmény publikus `bool`; a secret klasszifikáció nem szivárog át az összehasonlítás eredményére.

Ezek a műveletek nem hoznak létre authorization proofot. Egy account betöltése és jelszavának ellenőrzése/módosítása továbbra is a normál auth szerződéseket követi.
