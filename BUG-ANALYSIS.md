# Bug Analysis: Premature Thread Shutdown

## Issue Description

The database updates thread is shutting down prematurely with the message:
```
2026-01-20 12:25:25.776950431 [INFO][torrust_actix] [BOOT] Shutting down thread for updates...
```

This happens without a graceful shutdown signal (Ctrl+C) being sent.

## Root Cause Analysis

### Location
**File**: `src/main.rs:406-408`
```rust
_ = shutdown_waiting(Duration::from_secs(1), updates_handler.clone()) => {
    info!("[BOOT] Shutting down thread for updates...");
    return;
}
```

### The Problem

The `shutdown_waiting` function is designed to check if a shutdown signal has been received:

**File**: `src/common/common.rs`
```rust
pub async fn shutdown_waiting(timeout: Duration, shutdown_handler: Shutdown) -> bool {
    future::timeout(timeout, shutdown_handler.handle())
        .await
        .is_ok()
}
```

**What's happening**:
1. Every iteration of the tokio::select! loop, `shutdown_waiting` is called with a 1-second timeout
2. It waits for `shutdown_handler.handle()` to complete within that timeout
3. If the handle completes (returns `Ok`), it means shutdown was signaled
4. The thread then exits prematurely

**The bug**: The `shutdown_handler.handle()` future is completing when it shouldn't be. This could happen if:

1. **The tokio-shutdown Shutdown handle is being dropped somewhere** - When the main `Shutdown` instance is dropped, all clones' `handle()` futures complete
2. **The shutdown signal is being triggered accidentally** - Something is calling the shutdown trigger
3. **Race condition in tokio-shutdown crate** - The crate might have a bug with cloned handles

## Evidence

From `src/main.rs`:
- Line 90: `let tokio_shutdown = Shutdown::new().expect("shutdown creation works on first call");`
- Line 338: `let updates_handler = tokio_shutdown.clone();` - Clone for updates thread
- Line 429: `tokio_shutdown.handle().await;` - Main thread waits for shutdown

The same pattern is used for multiple threads:
- Line 229: `stats_handler` (stats thread)
- Line 294: `cleanup_handler` (peers cleanup)
- Line 312: `cleanup_keys_handler` (keys cleanup)
- Line 338: `updates_handler` (database updates)

**Question**: Are other threads also shutting down prematurely, or just the updates thread?

## Potential Fixes

### Option 1: Check if tokio_shutdown is being dropped (Most Likely)

The main `tokio_shutdown` instance might be getting dropped before the spawned tasks complete. Check if there's code that drops or moves the main shutdown instance.

**Fix**: Ensure `tokio_shutdown` lives for the entire program duration by keeping it in scope until all threads complete.

### Option 2: Use a different shutdown mechanism

Instead of relying on `tokio-shutdown` crate, use a simpler approach with `tokio::sync::watch`:

```rust
// At program start
let (shutdown_tx, _shutdown_rx) = tokio::sync::watch::channel(false);

// For each background thread
let mut shutdown_rx = shutdown_tx.subscribe();

// In the select! loop
_ = shutdown_rx.changed() => {
    if *shutdown_rx.borrow() {
        info!("[BOOT] Shutting down thread for updates...");
        return;
    }
}

// On Ctrl+C
let _ = shutdown_tx.send(true);
```

### Option 3: Increase timeout or remove the check

If the shutdown check isn't critical for the periodic interval:

```rust
// Instead of checking every 1 second
_ = shutdown_handler.handle() => {
    info!("[BOOT] Shutting down thread for updates...");
    return;
}
```

This waits indefinitely for shutdown instead of checking with a timeout.

### Option 4: Debug logging

Add debug logging to understand what's triggering the shutdown:

```rust
_ = shutdown_waiting(Duration::from_secs(1), updates_handler.clone()) => {
    warn!("[BOOT] Shutdown signal received for updates thread!");
    warn!("[BOOT] This should only happen on Ctrl+C or program termination");
    return;
}
```

Then check logs to see if this is happening during normal operation or only at startup.

## Recommended Investigation Steps

1. **Add debug logging** before the shutdown message to confirm it's the `shutdown_waiting` branch triggering
2. **Check if other background threads** (stats, cleanup, cleanup_keys) are also shutting down prematurely
3. **Verify tokio-shutdown version** in Cargo.toml - there might be a known bug
4. **Test with Option 3** (remove timeout) to see if the thread runs correctly
5. **Check for any code** that might be calling shutdown trigger methods

## Related Code Locations

- Main shutdown handler initialization: `src/main.rs:90`
- Updates thread spawn: `src/main.rs:343-412`
- Shutdown waiting function: `src/common/common.rs`
- Similar pattern in other threads:
  - Stats thread: `src/main.rs:229-290`
  - Cleanup thread: `src/main.rs:294-308`
  - Keys cleanup thread: `src/main.rs:312-334`

## Testing the Fix

After applying a fix, verify:

1. The database updates thread continues running
2. Periodic database saves complete successfully
3. Graceful shutdown (Ctrl+C) still works correctly
4. All background threads shut down properly on exit
5. No database connection leaks or hanging tasks

## Priority

**HIGH** - This bug prevents the database persistence feature from working correctly. Without the updates thread running, database changes are not being saved periodically.
