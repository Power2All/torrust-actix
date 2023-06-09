#!/bin/sh

if [ ! -z "${FULL_CONFIG}" ]
then
  cat << EOF > /root/torrust-actix/target/release/config.toml
${FULL_CONFIG}
EOF
else
  cat << EOF > /root/torrust-actix/target/release/config.toml
log_level = "${LOG_LEVEL}"
log_console_interval = ${LOG_CONSOLE_INTERVAL}
statistics_enabled = ${STATISTICS_ENABLED}
db_driver = "${DB_DRIVER}"
db_path = "${DB_PATH}"
persistence = ${PERSISTENCE}
persistence_interval = ${PERSISTENCE_INTERVAL}
api_key = "${API_KEY}"
whitelist = ${WHITELIST}
blacklist = ${BLACKLIST}
keys = ${KEYS}
keys_cleanup_interval = ${KEYS_CLEANUP_INTERVAL}
users = ${USERS}
interval = ${INTERVAL}
interval_minimum = ${INTERVAL_MINIMUM}
interval_cleanup = ${INTERVAL_CLEANUP}
peer_timeout = ${PEER_TIMEOUT}
peers_returned = ${PEERS_RETURNED}

[[udp_server]]
enabled = ${UDPV4_SERVER_ENABLED}
bind_address = "${UDPV4_SERVER_BIND_ADDRESS}"

[[udp_server]]
enabled = ${UDPV6_SERVER_ENABLED}
bind_address = "${UDPV6_SERVER_BIND_ADDRESS}"

[[http_server]]
enabled = ${TCPV4_SERVER_ENABLED}
bind_address = "${TCPV4_SERVER_BIND_ADDRESS}"
ssl = ${TCPV4_SERVER_SSL}
ssl_key = "${TCPV4_SERVER_SSL_KEY}"
ssl_cert = "${TCPV4_SERVER_SSL_CERT}"

[[http_server]]
enabled = ${TCPV6_SERVER_ENABLED}
bind_address = "${TCPV6_SERVER_BIND_ADDRESS}"
ssl = ${TCPV6_SERVER_SSL}
ssl_key = "${TCPV6_SERVER_SSL_KEY}"
ssl_cert = "${TCPV6_SERVER_SSL_CERT}"

[[api_server]]
enabled = ${TCPV4_API_ENABLED}
bind_address = "${TCPV4_API_BIND_ADDRESS}"
ssl = ${TCPV4_API_SSL}
ssl_key = "${TCPV4_API_SSL_KEY}"
ssl_cert = "${TCPV4_API_SSL_CERT}"

[[api_server]]
enabled = ${TCPV6_API_ENABLED}
bind_address = "${TCPV6_API_BIND_ADDRESS}"
ssl = ${TCPV6_API_SSL}
ssl_key = "${TCPV6_API_SSL_KEY}"
ssl_cert = "${TCPV6_API_SSL_CERT}"

[db_structure]
db_torrents = "${DB_STRUCTURE_DB_TORRENTS}"
table_torrents_info_hash = "${DB_STRUCTURE_TABLE_TORRENTS_INFO_HASH}"
table_torrents_completed = "${DB_STRUCTURE_TABLE_TORRENTS_COMPLETED}"
db_whitelist = "${DB_STRUCTURE_DB_WHITELIST}"
table_whitelist_info_hash = "${DB_STRUCTURE_TABLE_WHITELIST_INFO_HASH}"
db_blacklist = "${DB_STRUCTURE_DB_BLACKLIST}"
table_blacklist_info_hash = "${DB_STRUCTURE_TABLE_BLACKLIST_INFO_HASH}"
db_keys = "${DB_STRUCTURE_DB_KEYS}"
table_keys_hash = "${DB_STRUCTURE_TABLE_KEYS_HASH}"
table_keys_timeout = "${DB_STRUCTURE_TABLE_KEYS_TIMEOUT}"
db_users = "${DB_STRUCTURE_DB_USERS}"
table_users_uuid = "${DB_STRUCTURE_TABLE_USERS_UUID}"
table_users_key = "${DB_STRUCTURE_TABLE_USERS_KEY}"
table_users_uploaded = "${DB_STRUCTURE_TABLE_USERS_UPLOADED}"
table_users_downloaded = "${DB_STRUCTURE_TABLE_USERS_DOWNLOADED}"
table_users_completed = "${DB_STRUCTURE_TABLE_USERS_COMPLETED}"
table_users_updated = "${DB_STRUCTURE_TABLE_USERS_UPDATED}"
table_users_active = "${DB_STRUCTURE_TABLE_USERS_ACTIVE}"
EOF
fi

echo "Configuration:"
echo ""
cat /root/torrust-actix/target/release/config.toml
echo ""
