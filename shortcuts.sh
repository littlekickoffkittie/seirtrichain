#!/bin/bash
# Seirtrichain Quick Actions
# Source this file to use shortcuts: source shortcuts.sh

SIERTRI_DIR="/data/data/com.termux/files/home/seirtrichain"

# Wallet commands
alias wallet="cargo run --bin siertri-wallet --manifest-path=$SIERTRI_DIR/Cargo.toml"
alias wallet-new="cargo run --bin siertri-wallet-new --manifest-path=$SIERTRI_DIR/Cargo.toml"
alias wallet-backup="cargo run --bin siertri-wallet-backup --manifest-path=$SIERTRI_DIR/Cargo.toml"
alias wallet-restore="cargo run --bin siertri-wallet-restore --manifest-path=$SIERTRI_DIR/Cargo.toml"

# Transaction commands
alias send="cargo run --bin siertri-send --manifest-path=$SIERTRI_DIR/Cargo.toml"
alias balance="cargo run --bin siertri-balance --manifest-path=$SIERTRI_DIR/Cargo.toml"
alias history="cargo run --bin siertri-history --manifest-path=$SIERTRI_DIR/Cargo.toml"

# Mining commands
alias miner="cargo run --bin siertri-miner --manifest-path=$SIERTRI_DIR/Cargo.toml"
alias mine-block="cargo run --bin siertri-mine-block --manifest-path=$SIERTRI_DIR/Cargo.toml"

# Network commands
alias node="cargo run --bin siertri-node --manifest-path=$SIERTRI_DIR/Cargo.toml"
alias api="cargo run --bin siertri-api --manifest-path=$SIERTRI_DIR/Cargo.toml"

# Utility commands
alias addressbook="cargo run --bin siertri-addressbook --manifest-path=$SIERTRI_DIR/Cargo.toml"

# Release mode aliases (faster execution)
alias wallet-release="cargo run --release --bin siertri-wallet --manifest-path=$SIERTRI_DIR/Cargo.toml"
alias miner-release="cargo run --release --bin siertri-miner --manifest-path=$SIERTRI_DIR/Cargo.toml"
alias node-release="cargo run --release --bin siertri-node --manifest-path=$SIERTRI_DIR/Cargo.toml"
alias api-release="cargo run --release --bin siertri-api --manifest-path=$SIERTRI_DIR/Cargo.toml"

echo "Seirtrichain shortcuts loaded!"
echo "Available commands: wallet, wallet-new, wallet-backup, wallet-restore, send, balance, history, miner, mine-block, node, api, addressbook"
echo "Release mode: wallet-release, miner-release, node-release, api-release"
