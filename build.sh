#!/bin/sh

echo "Remove cache for clean build, without removing dynamic files like config..."
echo ""
rm -Rf target/release/.fingerprint target/release/build target/release/deps target/release/examples target/release/incremental target/release/.cargo-lock target/.rustc_info.json target/CACHEDIR.TAG

echo ""
echo "The full web URL to your tracker API ( e.g. http://tracker.domain.com:8080 ):"
read API_URL
API_URL=$(echo ${API_URL} | sed -e "s#/#\\\/#g")
sed -i "s/{TRACKER_URL}/${API_URL}/g" webgui/index.htm
echo ""
cargo build --release
sed -i "s/${API_URL}/{TRACKER_URL}/g" webgui/index.htm
echo ""
echo "#####################################################"
echo "Building completed !"
echo "You can find the compiled binary at ./target/release/"
echo "#####################################################"
