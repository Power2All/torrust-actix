use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use log::{info, warn};
use parking_lot::RwLock;
use crate::udp::structs::dynamic_queue::DynamicQueue;
use crate::udp::structs::udp_packet::UdpPacket;

impl DynamicQueue {
    pub fn new(initial_capacity: usize, segment_size: usize, max_segments: usize) -> Self {
        let initial_segments = initial_capacity.div_ceil(segment_size);
        let mut segments = Vec::with_capacity(initial_segments);

        for _ in 0..initial_segments {
            segments.push(Arc::new(crossbeam::queue::ArrayQueue::new(segment_size)));
        }

        DynamicQueue {
            segments: Arc::new(RwLock::new(segments)),
            current_write_segment: AtomicUsize::new(0),
            current_read_segment: AtomicUsize::new(0),
            segment_size,
            max_segments,
            total_capacity: AtomicUsize::new(initial_capacity),
            total_items: AtomicUsize::new(0),
            is_growing: AtomicBool::new(false),
        }
    }

    /// Push a packet, growing the queue if necessary
    pub fn push(&self, packet: UdpPacket) -> bool {
        // Try fast path first - push to current segment
        if let Some(success) = self.try_push_fast(&packet) {
            if success {
                self.total_items.fetch_add(1, Ordering::Relaxed);
                return true;
            }
        }

        // Slow path - might need to grow or find another segment
        self.push_slow(packet)
    }

    /// Fast path push attempt
    fn try_push_fast(&self, packet: &UdpPacket) -> Option<bool> {
        let segments = self.segments.read();
        let write_idx = self.current_write_segment.load(Ordering::Acquire);

        if write_idx < segments.len()
            && segments[write_idx].push(packet.clone()).is_ok() {
                return Some(true);
            }
        None
    }

    /// Slow path - handle full segments and growing
    fn push_slow(&self, packet: UdpPacket) -> bool {
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 3;

        while attempts < MAX_ATTEMPTS {
            // Try all existing segments
            {
                let segments = self.segments.read();
                let num_segments = segments.len();

                for i in 0..num_segments {
                    let idx = (self.current_write_segment.load(Ordering::Acquire) + i) % num_segments;
                    if segments[idx].push(packet.clone()).is_ok() {
                        self.total_items.fetch_add(1, Ordering::Relaxed);
                        // Update write segment hint for next time
                        self.current_write_segment.store(idx, Ordering::Release);
                        return true;
                    }
                }
            }

            // All segments full - try to grow
            if self.try_grow() {
                attempts += 1;
                continue;
            }

            // Can't grow - try harder to find space
            std::thread::yield_now();
            attempts += 1;
        }

        // Last resort - forcefully make space or drop
        self.emergency_push(packet)
    }

    /// Try to grow the queue by adding a new segment
    fn try_grow(&self) -> bool {
        // Check if another thread is already growing
        if self.is_growing.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            // Another thread is growing, wait for it
            while self.is_growing.load(Ordering::Acquire) {
                std::thread::yield_now();
            }
            return true;
        }

        // We got the lock to grow
        let mut segments = self.segments.write();
        let current_segments = segments.len();

        if current_segments >= self.max_segments {
            self.is_growing.store(false, Ordering::Release);
            warn!("Parse pool at maximum capacity: {current_segments} segments");
            return false;
        }

        // Add new segment
        segments.push(Arc::new(crossbeam::queue::ArrayQueue::new(self.segment_size)));
        let new_capacity = segments.len() * self.segment_size;
        self.total_capacity.store(new_capacity, Ordering::Release);

        info!("Parse pool grew: {} -> {} capacity ({} segments)",
              current_segments * self.segment_size,
              new_capacity,
              segments.len());

        self.is_growing.store(false, Ordering::Release);
        true
    }

    /// Emergency push - used when queue is completely full
    fn emergency_push(&self, packet: UdpPacket) -> bool {
        // Try one more time after potential growth
        if let Some(success) = self.try_push_fast(&packet) {
            if success {
                self.total_items.fetch_add(1, Ordering::Relaxed);
                return true;
            }
        }

        warn!("Parse pool full despite growth attempts, dropping packet");
        false
    }

    /// Pop a packet from the queue
    pub fn pop(&self) -> Option<UdpPacket> {
        // Try fast path first
        if let Some(packet) = self.try_pop_fast() {
            self.total_items.fetch_sub(1, Ordering::Relaxed);
            return Some(packet);
        }

        // Slow path - search all segments
        self.pop_slow()
    }

    /// Fast path pop attempt
    fn try_pop_fast(&self) -> Option<UdpPacket> {
        let segments = self.segments.read();
        let read_idx = self.current_read_segment.load(Ordering::Acquire);

        if read_idx < segments.len() {
            if let Some(packet) = segments[read_idx].pop() {
                return Some(packet);
            }
        }
        None
    }

    /// Slow path - search all segments for items
    fn pop_slow(&self) -> Option<UdpPacket> {
        let segments = self.segments.read();
        let num_segments = segments.len();

        for i in 0..num_segments {
            let idx = (self.current_read_segment.load(Ordering::Acquire) + i) % num_segments;
            if let Some(packet) = segments[idx].pop() {
                self.total_items.fetch_sub(1, Ordering::Relaxed);
                // Update read segment hint for next time
                self.current_read_segment.store(idx, Ordering::Release);
                return Some(packet);
            }
        }
        None
    }

    /// Get approximate number of items in the queue
    pub fn len(&self) -> usize {
        self.total_items.load(Ordering::Relaxed)
    }

    /// Returns true if the queue contains no items.
    /// This uses the same atomic counter as `len()` and is O(1).
    pub fn is_empty(&self) -> bool {
        self.total_items.load(Ordering::Relaxed) == 0
    }

    /// Get current capacity
    pub fn capacity(&self) -> usize {
        self.total_capacity.load(Ordering::Relaxed)
    }

    /// Get number of segments
    pub fn segments_count(&self) -> usize {
        self.segments.read().len()
    }

    /// Shrink queue if it has grown too large and is now mostly empty
    pub fn try_shrink(&self) {
        let segments = self.segments.read();
        let current_segments = segments.len();
        let initial_segments = 2; // Keep at least 2 segments

        if current_segments <= initial_segments {
            return;
        }

        // Check if we're using less than 25% capacity
        let items = self.len();
        let capacity = self.capacity();

        if items < capacity / 4 {
            drop(segments); // Release read lock
            self.shrink_to(initial_segments);
        }
    }

    /// Shrink to specified number of segments
    fn shrink_to(&self, target_segments: usize) {
        if self.is_growing.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            return; // Another operation in progress
        }

        let mut segments = self.segments.write();

        // Don't shrink if we have items in later segments
        for i in target_segments..segments.len() {
            if !segments[i].is_empty() {
                self.is_growing.store(false, Ordering::Release);
                return;
            }
        }

        // Safe to shrink
        let old_count = segments.len();
        segments.truncate(target_segments);
        let new_capacity = segments.len() * self.segment_size;
        self.total_capacity.store(new_capacity, Ordering::Release);

        info!("Parse pool shrank: {} -> {} segments", old_count, segments.len());

        self.is_growing.store(false, Ordering::Release);
    }
}