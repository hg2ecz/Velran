<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Dokumentációs státusz (2026-09-14):** Ellenőrzött fejlesztési mérföldkő. A jelenlegi forrásfán sikeresen lefutott a `cargo fmt`, a teljes workspace tesztkészlet, a `./verify.sh` és a helyi tesztkiszolgálás. Ez a repository-szintű fejlesztési baseline-t rögzíti; a környezetfüggő production deployment, recovery és operátori evidence továbbra is release-gate feladat.

# f32 FFT4096 benchmark

A példa teljes, 4096 mintás radix-2 FFT-t hajt végre Velran kódban `f32` tömbökön; nem hív natív FFT könyvtárat.

A determinisztikus bemenet a 64-es és 256-os binhez tartozó két szinuszt tartalmaz. Normalizálatlan FFT esetén a várt magnitúdók megközelítőleg 2048 és 1024. Az oldal `std::time::Instant::now()` segítségével csak az FFT szakaszt méri, és széles correctness tartományt is ellenőriz.

A compute-heavy példa futtatásához megfelelően magas instruction budget szükséges. Az első kérés lefordíthatja és betöltheti az érintett natív shardot; steady-state teljesítményméréshez a cache-hit ismételt kéréseket érdemes mérni.
