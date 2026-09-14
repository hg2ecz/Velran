<!-- VELRAN-DOC-STATUS: 2026-09-14 -->
> **Documentation status (2026-09-14):** Verified Development Milestone. On the current source tree, `cargo fmt`, the full workspace test suite, `./verify.sh`, and local test serving have completed successfully. This records the repository-level development baseline; environment-specific production deployment, recovery, and operational evidence remain release-gate responsibilities.

# A-to-Z mini product list

This is the book's minimal end-to-end example application.

1. Create a secret file containing an absolute SQLite URL, for example `sqlite:///tmp/velran-a-z-products.db`.
2. Check the source with `velran-cli check main.vrn`.
3. Apply the migration:
   `velran-cli migrate apply --dir migrations --db-url-file dev-db-url`.
4. Adjust the absolute paths in `server.toml.example` for your machine.
5. Start the server with `velran-server --config server.toml`.
6. Open `http://127.0.0.1:8080/`.

Do not commit the development DB URL file when it contains a real credential.

The Hungarian version is available as `README_hu.md`.
