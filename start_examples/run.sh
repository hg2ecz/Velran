#!/bin/bash

echo -n "appdir (see velran-server.toml):  "
grep "app" < velran-server.toml

velran-server --config ./velran-server.toml --check-config
echo
velran-server --config ./velran-server.toml
