#!/bin/sh

echo "###########################################################################"
echo "Remove cache for clean build, without removing dynamic files like config..."
echo "###########################################################################"

rm -Rf target/release/.fingerprint target/release/build target/release/deps target/release/examples target/release/incremental target/release/.cargo-lock target/.rustc_info.json target/CACHEDIR.TAG

echo "############################"
echo "Build Torrust-Axum - Release"
echo "############################"

cargo build --release

echo "####################"
echo "Building completed !"
echo "####################"
