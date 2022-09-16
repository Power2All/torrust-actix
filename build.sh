#!/bin/sh

echo "###########################################################################"
echo "Remove cache for clean build, without removing dynamic files like config..."
echo "###########################################################################"

rm -Rf target/release/.fingerprint target/release/build target/release/deps target/release/examples target/release/incremental target/release/.cargo-lock target/.rustc_info.json target/CACHEDIR.TAG

echo "############################"
echo "Build Torrust-Axum - Release"
echo "############################"

echo "The full web URL to your tracker API ( e.g. http://tracker.domain.com:8080 ):"
read apiurl
sed -i "s/{TRACKER_URL}/${apiurl}/g" webgui/index.htm

cargo build --release

sed -i "s/${apiurl}/{TRACKER_URL}/g" webgui/index.htm

echo "#####################################################"
echo "Building completed !"
echo "You can find the compiled binary at ./target/release/"
echo "#####################################################"
