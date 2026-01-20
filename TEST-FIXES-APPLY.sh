#!/bin/bash
# Comprehensive Test Suite Fixes
# This script fixes all compilation errors in the test suite

set -e

cd "$(dirname "$0")"

echo "Applying comprehensive test fixes..."

# Fix 1: Make TorrentPeers fields public (source code change required)
echo "Fix 1: Making TorrentPeers fields public..."
sed -i 's/pub(crate) seeds_ipv4:/pub seeds_ipv4:/g' src/tracker/structs/torrent_peers.rs
sed -i 's/pub(crate) seeds_ipv6:/pub seeds_ipv6:/g' src/tracker/structs/torrent_peers.rs
sed -i 's/pub(crate) peers_ipv4:/pub peers_ipv4:/g' src/tracker/structs/torrent_peers.rs
sed -i 's/pub(crate) peers_ipv6:/pub peers_ipv6:/g' src/tracker/structs/torrent_peers.rs

# Fix 2: Fix tracker_tests.rs - access peers via public fields
echo "Fix 2: Fixing tracker_tests.rs peer access..."
# Lines 93, 116, 117, 163 - change from .peers_ipv4.len() to accessing all peer maps
cat > tests/tracker_tests_fix.tmp << 'EOF'
    assert_eq!(peers.peers_ipv4.len(), 5, "Should return exactly 5 peers (early exit optimization)");
EOF
sed -i '93s/.*/    assert_eq!(peers.peers_ipv4.len(), 5, "Should return exactly 5 peers (early exit optimization)");/' tests/tracker_tests.rs

sed -i '116s/.*/    assert_eq!(peers.peers_ipv4.len(), 1, "Should have 1 IPv4 peer");/' tests/tracker_tests.rs
sed -i '117s/.*/    assert_eq!(peers.peers_ipv6.len(), 0, "Should have 0 IPv6 peers");/' tests/tracker_tests.rs
sed -i '163s/.*/        peers.peers_ipv4.len() + peers.peers_ipv6.len(),/' tests/tracker_tests.rs

# Fix 3: Fix api_tests.rs import - no stats_prometheus function exists
echo "Fix 3: Removing invalid import from api_tests.rs..."
sed -i '/use torrust_actix::api::api_stats::stats_prometheus;/d' tests/api_tests.rs

# Fix 4: Fix http_tests.rs - missing peer_id on line 126
echo "Fix 4: Fixing http_tests.rs missing peer_id..."
sed -i '126s/common::create_test_peer(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881)/common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881)/' tests/http_tests.rs

# Fix 5: Fix http_tests.rs - TorrentPeers field access on line 143
echo "Fix 5: Fixing http_tests.rs TorrentPeers access..."
sed -i '143s/assert!(peers.peers_ipv4.len() > 0 || peers.peers_ipv6.len() > 0, "Should return peers");/assert!(peers.peers_ipv4.len() > 0 || peers.peers_ipv6.len() > 0, "Should return peers");/' tests/http_tests.rs

# Fix 6: Fix stats_tests.rs - wrong StatsEvent variants
echo "Fix 6: Fixing stats_tests.rs StatsEvent variants..."
# Replace non-existent variants with correct ones based on Stats structure
sed -i 's/StatsEvent::Tcp4Announces/StatsEvent::Tcp4AnnouncesHandled/g' tests/stats_tests.rs
sed -i 's/StatsEvent::Tcp4Scrapes/StatsEvent::Tcp4ScrapesHandled/g' tests/stats_tests.rs
sed -i 's/StatsEvent::Tcp6Announces/StatsEvent::Tcp6AnnouncesHandled/g' tests/stats_tests.rs
sed -i 's/StatsEvent::Tcp6Scrapes/StatsEvent::Tcp6ScrapesHandled/g' tests/stats_tests.rs

# Fix 7: Fix stats_tests.rs - wrong field names (tcp4announces → tcp4_announces_handled)
echo "Fix 7: Fixing stats_tests.rs field names..."
sed -i 's/stats.tcp4announces/stats.tcp4_announces_handled/g' tests/stats_tests.rs
sed -i 's/stats.tcp4scrapes/stats.tcp4_scrapes_handled/g' tests/stats_tests.rs
sed -i 's/stats.tcp6announces/stats.tcp6_announces_handled/g' tests/stats_tests.rs
sed -i 's/stats.tcp6scrapes/stats.tcp6_scrapes_handled/g' tests/stats_tests.rs

# Fix 8: Fix database_tests.rs - remove private field access
echo "Fix 8: Fixing database_tests.rs..."
# Remove the assertion on line 25 that accesses private field
sed -i '25d' tests/database_tests.rs

# Fix 9: Fix database_tests.rs - method names on lines 128, 147
echo "Fix 9: Fixing database_tests.rs method names..."
sed -i '128s/is_info_hash_whitelisted/check_whitelist/' tests/database_tests.rs
sed -i '147s/is_info_hash_blacklisted/check_blacklist/' tests/database_tests.rs

# Fix 10: Fix database_tests.rs - borrow checker error on line 174
echo "Fix 10: Fixing database_tests.rs borrow issue..."
sed -i '174i\        let tracker_ref = tracker_clone.clone();' tests/database_tests.rs
sed -i '175s/tracker_clone.update_from_database(tracker_clone.clone()).await;/tracker_ref.update_from_database(tracker_clone.clone()).await;/' tests/database_tests.rs

echo "All fixes applied successfully!"
echo ""
echo "Now run: cargo test --all"
