# Fix Thread Shutdown Bug - Step by Step

## The Problem

Your database updates thread is exiting prematurely with:
```
[INFO] [BOOT] Shutting down thread for updates...
```

This prevents database persistence from working.

## The Fix (5 minutes)

### Option 1: Remove Timeout (Recommended)

**File**: `src/main.rs`
**Line**: 406

**Change this:**
```rust
_ = shutdown_waiting(Duration::from_secs(1), updates_handler.clone()) => {
    info!("[BOOT] Shutting down thread for updates...");
    return;
}
```

**To this:**
```rust
_ = updates_handler.handle() => {
    info!("[BOOT] Shutting down thread for updates...");
    return;
}
```

**Why**: Removes the 1-second timeout check that's completing prematurely. Now it will only exit when an actual shutdown signal is received.

### Apply the Fix

```bash
cd C:\Coding\torrust-actix\src

# Backup first
cp main.rs main.rs.backup

# Apply fix (PowerShell):
(Get-Content main.rs) -replace '_\s*=\s*shutdown_waiting\(Duration::from_secs\(1\),\s*updates_handler\.clone\(\)\)\s*=>', '_ = updates_handler.handle() =>' | Set-Content main.rs
```

Or manually edit line 406 in `src/main.rs`.

### Test the Fix

```bash
cd C:\Coding\torrust-actix

# Rebuild
cargo build --release

# Run and monitor logs
cargo run --release 2>&1 | grep -E "(BOOT|DATABASE)"
```

**Expected output:**
```
[INFO] [BOOT] Starting thread for database updates with 60 seconds delay...
[INFO] [DATABASE UPDATES] Starting batch updates...
[INFO] [DATABASE UPDATES] Batch updates completed
... (repeats every 60 seconds)
```

**Should NOT see** (unless you press Ctrl+C):
```
[INFO] [BOOT] Shutting down thread for updates...
```

### Verify It Works

1. Let the app run for 2-3 minutes
2. Check that database updates are happening periodically
3. Press Ctrl+C to gracefully shutdown
4. Verify all threads shut down properly

## Option 2: Add Debug Logging (If Option 1 doesn't work)

If the thread still exits, add debug logging to understand why:

**In `src/main.rs` line 406**, add:
```rust
_ = updates_handler.handle() => {
    warn!("[DEBUG] Shutdown signal received in updates thread!");
    warn!("[DEBUG] This should only happen on Ctrl+C");
    warn!("[DEBUG] Stack trace: {:?}", std::backtrace::Backtrace::capture());
    info!("[BOOT] Shutting down thread for updates...");
    return;
}
```

Then check logs to see what's triggering the shutdown.

## Option 3: Check All Background Threads

The same issue might affect other threads. Check these locations in `src/main.rs`:

1. **Line 286** - Stats thread
2. **Line 306** - Cleanup thread
3. **Line 332** - Keys cleanup thread
4. **Line 406** - Updates thread (this one)

Apply the same fix to all of them:

```bash
# Find all occurrences
grep -n "shutdown_waiting" src/main.rs
```

Replace each one with the direct `.handle()` call.

## Commit the Fix

Once verified:

```bash
git add src/main.rs
git commit -m "Fix premature thread shutdown in background tasks

Remove 1-second timeout from shutdown_waiting calls.
The timeout was causing threads to exit prematurely.
Now threads only exit on actual shutdown signals.

Fixes database persistence and background task issues.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

## If Still Having Issues

Check if `tokio_shutdown` is being dropped:

```rust
// In main.rs around line 90
let tokio_shutdown = Shutdown::new().expect("shutdown creation works on first call");

// Make sure this lives until the end of main()
// Add at the very end of main, after line 429:
let _ = tokio_shutdown; // Keep alive
```

## Success Criteria

✅ Database updates thread stays running
✅ Periodic updates happen (check logs)
✅ Graceful shutdown works (Ctrl+C)
✅ No premature "Shutting down thread" messages
✅ Database changes persist between restarts

## Next Steps

After fixing this:
1. Test database persistence thoroughly
2. Verify all background tasks work
3. Return to fixing test compilation (see QUICK-FIX.md)
