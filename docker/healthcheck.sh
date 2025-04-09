#!/bin/sh

HC_API=$(grep '\[\[api_server\]\]' target/release/config.toml -A 12 | grep "enabled " | cut -d "=" -f 2 | tr -d '[:space:]\"' | tr '[:upper:]' '[:lower:]')
HC_API_ADDRESS=$(grep '\[\[api_server\]\]' target/release/config.toml -A 12 | grep "bind_address " | cut -d "=" -f 2 | tr -d '[:space:]\"' | tr '[:upper:]' '[:lower:]')
HC_API_SSL=$(grep '\[\[api_server\]\]' target/release/config.toml -A 12 | grep "ssl " | cut -d "=" -f 2 | tr -d '[:space:]\:' | tr '[:upper:]' '[:lower:]')
HC_HTTP=$(grep '\[\[http_server\]\]' target/release/config.toml -A 12 | grep "enabled " | cut -d "=" -f 2 | tr -d '[:space:]\:' | tr '[:upper:]' '[:lower:]')
HC_HTTP_ADDRESS=$(grep '\[\[http_server\]\]' target/release/config.toml -A 12 | grep "bind_address " | cut -d "=" -f 2 | tr -d '[:space:]\"' | tr '[:upper:]' '[:lower:]')
HC_HTTP_SSL=$(grep '\[\[http_server\]\]' target/release/config.toml -A 12 | grep "ssl " | cut -d "=" -f 2 | tr -d '[:space:]\"' | tr '[:upper:]' '[:lower:]')
HC_UDP=$(grep '\[\[udp_server\]\]' target/release/config.toml -A 4 | grep "enabled " | cut -d "=" -f 2 | tr -d '[:space:]\"' | tr '[:upper:]' '[:lower:]')
HC_UDP_ADDRESS=$(grep '\[\[udp_server\]\]' target/release/config.toml -A 4 | grep "bind_address " | cut -d "=" -f 2 | tr -d '[:space:]\"' | tr '[:upper:]' '[:lower:]')

printf "[API]: %s - %s - %s\n" "${HC_API}" "${HC_API_ADDRESS}" "${HC_API_SSL}"
printf "[HTTP]: %s - %s - %s\n" "${HC_HTTP}" "${HC_HTTP_ADDRESS}" "${HC_HTTP_SSL}"
printf "[UDP]: %s - %s\n" "${HC_UDP}" "${HC_UDP_ADDRESS}"