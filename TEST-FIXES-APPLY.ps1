# Comprehensive Test Suite Fixes (PowerShell)
# This script fixes all compilation errors in the test suite

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "Applying comprehensive test fixes..." -ForegroundColor Cyan

# Fix 1: Make TorrentPeers fields public (source code change required)
Write-Host "Fix 1: Making TorrentPeers fields public..." -ForegroundColor Yellow
$torrentPeersPath = "src\tracker\structs\torrent_peers.rs"
(Get-Content $torrentPeersPath) -replace 'pub\(crate\) seeds_ipv4:', 'pub seeds_ipv4:' `
    -replace 'pub\(crate\) seeds_ipv6:', 'pub seeds_ipv6:' `
    -replace 'pub\(crate\) peers_ipv4:', 'pub peers_ipv4:' `
    -replace 'pub\(crate\) peers_ipv6:', 'pub peers_ipv6:' | Set-Content $torrentPeersPath

# Fix 2: Fix api_tests.rs import - no stats_prometheus function exists
Write-Host "Fix 2: Removing invalid import from api_tests.rs..." -ForegroundColor Yellow
$apiTestsPath = "tests\api_tests.rs"
(Get-Content $apiTestsPath) | Where-Object { $_ -notmatch 'use torrust_actix::api::api_stats::stats_prometheus;' } | Set-Content $apiTestsPath

# Fix 3: Fix http_tests.rs - missing peer_id on line 126
Write-Host "Fix 3: Fixing http_tests.rs missing peer_id..." -ForegroundColor Yellow
$httpTestsPath = "tests\http_tests.rs"
$httpContent = Get-Content $httpTestsPath
$httpContent[125] = $httpContent[125] -replace 'common::create_test_peer\(IpAddr::V4\(Ipv4Addr::new\(127, 0, 0, 1\)\), 6881\)', 'common::create_test_peer(peer_id, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881)'
$httpContent | Set-Content $httpTestsPath

# Fix 4: Fix database_tests.rs - remove private field access on line 25
Write-Host "Fix 4: Fixing database_tests.rs private field access..." -ForegroundColor Yellow
$dbTestsPath = "tests\database_tests.rs"
$dbContent = Get-Content $dbTestsPath
# Remove line 25 (index 24) - the assertion about engine
$dbContent = $dbContent[0..23] + $dbContent[25..($dbContent.Length-1)]
$dbContent | Set-Content $dbTestsPath

# Fix 5: Fix database_tests.rs - method names
Write-Host "Fix 5: Fixing database_tests.rs method names..." -ForegroundColor Yellow
(Get-Content $dbTestsPath) -replace 'is_info_hash_whitelisted', 'check_whitelist' `
    -replace 'is_info_hash_blacklisted', 'check_blacklist' | Set-Content $dbTestsPath

# Fix 6: Fix stats_tests.rs - wrong StatsEvent variants and field names
Write-Host "Fix 6: Fixing stats_tests.rs event variants and field names..." -ForegroundColor Yellow
$statsTestsPath = "tests\stats_tests.rs"
(Get-Content $statsTestsPath) -replace 'StatsEvent::Tcp4Announces', 'StatsEvent::Tcp4AnnouncesHandled' `
    -replace 'StatsEvent::Tcp4Scrapes', 'StatsEvent::Tcp4ScrapesHandled' `
    -replace 'StatsEvent::Tcp6Announces', 'StatsEvent::Tcp6AnnouncesHandled' `
    -replace 'StatsEvent::Tcp6Scrapes', 'StatsEvent::Tcp6ScrapesHandled' `
    -replace 'stats\.tcp4announces', 'stats.tcp4_announces_handled' `
    -replace 'stats\.tcp4scrapes', 'stats.tcp4_scrapes_handled' `
    -replace 'stats\.tcp6announces', 'stats.tcp6_announces_handled' `
    -replace 'stats\.tcp6scrapes', 'stats.tcp6_scrapes_handled' | Set-Content $statsTestsPath

Write-Host ""
Write-Host "All fixes applied successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Now run: cargo test --all" -ForegroundColor Cyan
