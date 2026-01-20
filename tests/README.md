## Test Suite for Torrust-Actix

This directory contains comprehensive integration tests for the Torrust-Actix BitTorrent tracker.

### Test Organization

```
tests/
├── common/          # Shared test utilities and fixtures
│   └── mod.rs       # Helper functions for creating test data
├── tracker_tests.rs # Core tracker functionality tests
├── database_tests.rs# Database layer tests (SQLite/MySQL/PostgreSQL)
├── udp_tests.rs     # UDP protocol implementation tests
├── http_tests.rs    # HTTP tracker protocol tests
├── api_tests.rs     # RESTful API endpoint tests
├── config_tests.rs  # Configuration management tests
├── stats_tests.rs   # Statistics and metrics tests
└── README.md        # This file

benches/
└── tracker_benchmarks.rs  # Performance benchmarks
```

### Running Tests

#### All Tests
```bash
cargo test
```

#### Specific Test Module
```bash
cargo test tracker_tests
cargo test database_tests
cargo test udp_tests
cargo test http_tests
cargo test api_tests
cargo test config_tests
cargo test stats_tests
```

#### Single Test
```bash
cargo test test_add_peer_to_new_torrent
```

#### With Output
```bash
cargo test -- --nocapture
```

#### Parallel vs Sequential
```bash
# Run tests in parallel (default, faster)
cargo test

# Run tests sequentially (useful for database tests)
cargo test -- --test-threads=1
```

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench bench_add_peer

# Save baseline for comparison
cargo bench -- --save-baseline my-baseline

# Compare against baseline
cargo bench -- --baseline my-baseline
```

### Test Coverage

#### Core Tracker (tracker_tests.rs)
- ✅ Add peer to new torrent
- ✅ Add seed to torrent
- ✅ Peer to seed transition (completed event)
- ✅ Remove peer from torrent
- ✅ Get peers with limit (early exit optimization)
- ✅ IPv4/IPv6 filtering
- ✅ Torrent sharding distribution
- ✅ Concurrent peer additions (thread safety)
- ✅ Statistics tracking
- ✅ Whitelist filtering
- ✅ Blacklist filtering

#### Database Layer (database_tests.rs)
- ✅ Database connector creation
- ✅ Load torrents from empty database
- ✅ Save and load whitelist
- ✅ Save and load blacklist
- ✅ Save and load keys
- ✅ Optimization validation (no clone/unwrap)
- ✅ Update actions (Add/Remove)
- ✅ Reset seeds/peers
- ✅ Concurrent database writes

#### UDP Protocol (udp_tests.rs)
- ✅ Connect request parsing
- ✅ Malformed packet handling
- ✅ Connect response writing
- ✅ Zero-copy optimization validation
- ✅ Announce request parsing
- ✅ Scrape request parsing
- ✅ Packet size limits (MAX_SCRAPE_TORRENTS)
- ✅ Response size estimation
- ✅ Connection ID generation
- ✅ Protocol identifier constant

#### HTTP Protocol (http_tests.rs)
- ✅ Announce endpoint
- ✅ Scrape endpoint
- ✅ CORS headers
- ✅ Invalid endpoint (404)
- ✅ IPv6 support
- ✅ Server cleanup optimization

#### API Endpoints (api_tests.rs)
- ✅ Prometheus metrics endpoint
- ✅ Torrent deletion API
- ✅ Whitelist management API
- ✅ Blacklist management API
- ✅ Key management API
- ✅ CORS header validation
- ✅ 404 error handling
- ✅ Content-Type verification
- ✅ Concurrent API operations

#### Configuration (config_tests.rs)
- ✅ Default configuration values
- ✅ TOML file loading
- ✅ Database settings validation
- ✅ Tracker limits validation
- ✅ UDP server configuration
- ✅ HTTP server configuration
- ✅ Sentry integration defaults
- ✅ Configuration validation ranges
- ✅ Thread-safe Arc cloning
- ✅ Concurrent configuration access

#### Statistics (stats_tests.rs)
- ✅ Initial statistics values
- ✅ Increment/decrement operations
- ✅ Concurrent stats updates
- ✅ Completed downloads tracking
- ✅ Seed/peer ratio tracking
- ✅ Prometheus format output
- ✅ Atomic counter operations
- ✅ Torrent lifecycle stats
- ✅ Overflow protection
- ✅ HTTP/TCP stats separation

### Performance Benchmarks

The benchmark suite validates the optimizations applied:

1. **Add Peer** - Baseline peer addition performance
2. **Get Peers with Early Exit** - Tests the early exit optimization for peer limits
3. **Concurrent Peer Additions** - Thread safety and concurrent performance
4. **Sharding Distribution** - Validates 256-shard architecture
5. **UDP Packet Parsing** - Zero-copy optimization for UDP packets
6. **Peer Filtering** - IPv4/IPv6 filtering performance

### Continuous Integration

Tests run automatically on:
- Every push to main branch
- Every pull request
- Scheduled nightly runs

See `.github/workflows/tests.yml` for CI configuration.

### Test Dependencies

The following dev-dependencies are used:
- `tempfile` - Temporary directories for database tests
- `rand` - Random data generation for tests
- `criterion` - Benchmarking framework
- `mockall` - Mocking framework (for future use)
- `proptest` - Property-based testing (for future use)

### Writing New Tests

#### Example Test Structure

```rust
#[tokio::test]
async fn test_my_feature() {
    // Arrange
    let tracker = common::create_test_tracker().await;
    let info_hash = common::random_info_hash();

    // Act
    let result = tracker.some_operation(info_hash);

    // Assert
    assert!(result.is_ok(), "Operation should succeed");
}
```

#### Best Practices

1. **Use common utilities** - Reuse helper functions from `common/mod.rs`
2. **Clean test data** - Use tempfile for temporary files
3. **Test isolation** - Each test should be independent
4. **Meaningful assertions** - Include descriptive assertion messages
5. **Test both success and failure** - Cover error cases
6. **Benchmark regressions** - Add benchmarks for performance-critical code

### Troubleshooting

#### Database Tests Fail
- Ensure SQLite is available
- Check file permissions for temporary directories
- Try running with `--test-threads=1`

#### UDP Tests Fail
- Check if ports are available
- Verify firewall settings
- Ensure network connectivity

#### Benchmarks Show Regressions
- Compare against saved baseline: `cargo bench -- --baseline <name>`
- Check for system load during benchmarking
- Run multiple times to average out noise

### Future Enhancements

- [ ] Property-based testing with proptest
- [ ] Fuzz testing for protocol parsers
- [ ] Load testing with realistic traffic patterns
- [ ] MySQL/PostgreSQL integration tests (requires running servers)
- [ ] Stress tests for memory leaks
- [ ] Coverage reports with tarpaulin
- [ ] End-to-end BitTorrent client integration tests
