#!/bin/bash

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[1;36m'
PURPLE='\033[0;35m'
NC='\033[0m'

SSH_TARGET="pmos"
PKG_NAME="$(grep -P "name\\s?=\\s?\"([\w-]+)\"" Cargo.toml | cut -d"\"" -f2)"
CROSS_TRIPLE="aarch64-unknown-linux-musl"
TARGET_DIR="/tmp"

printf "===> ${CYAN}Cross compiling ${YELLOW}$PKG_NAME${CYAN} for $SSH_TARGET${NC}\n"
printf "===> ${CYAN}Using triple ${YELLOW}$CROSS_TRIPLE${NC}\n\n"

if ! $(cargo install --list | grep -q "cross"); then
    printf "===> ${CYAN}Installing cross{NC}\n"
    cargo install -f cross
fi

cross build --target aarch64-unknown-linux-musl --color always

BINARIES=$(find target/"$CROSS_TRIPLE"/ -maxdepth 2 -type f -executable -printf "%f ")
PATHS=$(find target/"$CROSS_TRIPLE"/ -maxdepth 2 -type f -executable -printf "%p ")

printf "===> ${CYAN}Copying binaries ${PURPLE}(${YELLOW}$(echo $BINARIES | sed 's/ /, /')${PURPLE})${CYAN} to ${YELLOW}$SSH_TARGET${CYAN}:${RED}$TARGET_DIR${NC}\n"
scp -o LogLevel=ERROR $PATHS "$SSH_TARGET":"$TARGET_DIR"/

BIN=$(echo $BINARIES | cut -d" " -f1)
printf "===> ${CYAN}Running ${YELLOW}$BIN${CYAN} on ${YELLOW}$SSH_TARGET${NC}\n"
set -x
ssh -t -o LogLevel=ERROR "$SSH_TARGET" "sudo $TARGET_DIR/$BIN"
