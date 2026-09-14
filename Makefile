all:
	cargo build --release
	strip target/release/velran-server
	strip target/release/velran-cli

install:
	sudo install -p 0755 target/release/velran-server /usr/local/bin
	sudo install -p 0755 target/release/velran-cli    /usr/local/bin
	sudo install -p 0755 config/server.toml.sample    /usr/local/etc/velran/
