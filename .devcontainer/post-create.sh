#!/bin/sh

sudo usermod -a -G docker vscode
sudo chmod 777 /sccache/

sudo apt-get update
sudo apt-get install -y  postgresql-client

unset RUSTC_WRAPPER
cargo install sccache --version ^0.9
cargo install sqlx-cli@0.8.3 --no-default-features --features postgres