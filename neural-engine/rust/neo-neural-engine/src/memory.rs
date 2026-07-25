use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use parking_lot::Mutex;

use crate::error::{NeuralError, NeuralResult};

/// Represents memory placement location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryLocation {
    Host,
    Device(u32),
    Pinned,
    Unified,
}

impl fmt::Display for MemoryLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host => write!(f, "Host"),
            Self::Device(id) => write!(f, "Device({})", id),
            Self::Pinned => write!(f, "Pinned"),
            Self::Unified => write!(f, "Unified"),
        }
    }
}

/// A reference-counted memory block backed by Vec<u8>.
#[derive(Debug)]
pub struct MemoryBlock {
    data: Vec<u8>,
    location: MemoryLocation,
    ref_count: AtomicUsize,
}

impl MemoryBlock {
    /// Allocates a new memory block of the given size and location.
    pub fn new(size: usize, location: MemoryLocation) -> NeuralResult<Self> {
        if size == 0 {
            return Err(NeuralError::MemoryAllocation {
                requested: 0,
                available: 0,
                context: "cannot allocate zero bytes".to_string(),
            });
        }

        Ok(Self {
            data: vec![0u8; size],
            location,
            ref_count: AtomicUsize::new(1),
        })
    }

    /// Returns the size of this block in bytes.
    #[must_use]
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Returns the memory location.
    #[must_use]
    pub fn location(&self) -> MemoryLocation {
        self.location
    }

    /// Returns a byte slice of the block's data.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Returns a mutable byte slice of the block's data.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Returns a pointer to the underlying data.
    #[must_use]
    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    /// Returns a mutable pointer to the underlying data.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_mut_ptr()
    }

    /// Increments the reference count.
    pub fn increment_refs(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrements the reference count. Returns true if this was the last reference.
    #[must_use]
    pub fn decrement_refs(&self) -> bool {
        self.ref_count.fetch_sub(1, Ordering::AcqRel) == 1
    }

    /// Returns the current reference count.
    #[must_use]
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::Relaxed)
    }
}

/// A thread-safe arena allocator for fast temporary allocations.
#[derive(Debug)]
pub struct ArenaAllocator {
    blocks: Mutex<Vec<ArenaBlock>>,
    block_size: usize,
    current_offset: AtomicUsize,
    total_allocated: AtomicUsize,
}

#[derive(Debug)]
struct ArenaBlock {
    data: Vec<u8>,
}

impl ArenaAllocator {
    /// Creates a new arena allocator with the given block size.
    #[must_use]
    pub fn new(block_size: usize) -> Self {
        let initial = vec![0u8; block_size];
        Self {
            blocks: Mutex::new(vec![ArenaBlock { data: initial }]),
            block_size,
            current_offset: AtomicUsize::new(0),
            total_allocated: AtomicUsize::new(0),
        }
    }

    /// Allocates `size` bytes from the arena. Returns the offset within the current block.
    pub fn allocate(&self, size: usize) -> NeuralResult<usize> {
        let offset = self.current_offset.load(Ordering::Relaxed);
        let new_offset = offset + size;

        if new_offset > self.block_size {
            let mut blocks = self.blocks.lock();
            let actual_block_size = self.block_size.max(size);
            blocks.push(ArenaBlock {
                data: vec![0u8; actual_block_size],
            });
            drop(blocks);
            self.current_offset.store(size, Ordering::Relaxed);
            self.total_allocated
                .fetch_add(actual_block_size, Ordering::Relaxed);
            Ok(0)
        } else {
            self.current_offset
                .store(new_offset, Ordering::Relaxed);
            self.total_allocated.fetch_add(size, Ordering::Relaxed);
            Ok(offset)
        }
    }

    /// Resets the arena, freeing nothing but allowing reuse.
    pub fn reset(&self) {
        self.current_offset.store(0, Ordering::Relaxed);
    }

    /// Returns total bytes allocated through this arena.
    #[must_use]
    pub fn total_allocated(&self) -> usize {
        self.total_allocated.load(Ordering::Relaxed)
    }

    /// Returns the block size.
    #[must_use]
    pub fn block_size(&self) -> usize {
        self.block_size
    }
}

impl Default for ArenaAllocator {
    fn default() -> Self {
        Self::new(64 * 1024 * 1024) // 64MB default block
    }
}

/// Memory pool for managing fixed-size blocks.
#[derive(Debug)]
pub struct MemoryPool {
    block_size: usize,
    total_blocks: usize,
    free_offsets: Mutex<Vec<usize>>,
    allocated: AtomicUsize,
}

impl MemoryPool {
    /// Creates a new memory pool with `num_blocks` of `block_size` bytes each.
    #[must_use]
    pub fn new(block_size: usize, num_blocks: usize) -> Self {
        let mut free = Vec::with_capacity(num_blocks);
        for i in 0..num_blocks {
            free.push(i * block_size);
        }
        Self {
            block_size,
            total_blocks: num_blocks,
            free_offsets: Mutex::new(free),
            allocated: AtomicUsize::new(0),
        }
    }

    /// Allocates a block from the pool, returning the byte offset.
    pub fn allocate(&self) -> NeuralResult<usize> {
        self.allocated.fetch_add(1, Ordering::Relaxed);
        let mut free = self.free_offsets.lock();
        free.pop().ok_or_else(|| {
            self.allocated.fetch_sub(1, Ordering::Relaxed);
            NeuralError::MemoryAllocation {
                requested: self.block_size,
                available: 0,
                context: "memory pool exhausted".to_string(),
            }
        })
    }

    /// Returns a block to the pool.
    pub fn free(&self, offset: usize) {
        let mut free = self.free_offsets.lock();
        free.push(offset);
        self.allocated.fetch_sub(1, Ordering::Relaxed);
    }

    /// Returns the number of free blocks.
    #[must_use]
    pub fn free_count(&self) -> usize {
        self.free_offsets.lock().len()
    }

    /// Returns the number of allocated blocks.
    #[must_use]
    pub fn allocated_count(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }

    /// Returns total pool size in bytes.
    #[must_use]
    pub fn total_bytes(&self) -> usize {
        self.block_size * self.total_blocks
    }

    /// Returns the block size.
    #[must_use]
    pub fn block_size(&self) -> usize {
        self.block_size
    }
}

/// A managed memory region with tracking and protection.
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    id: u64,
    offset: usize,
    size: usize,
    location: MemoryLocation,
    #[allow(dead_code)]
    protection: MemoryProtection,
}

/// Memory protection flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryProtection {
    Read,
    ReadWrite,
    ReadExecute,
    NoAccess,
}

impl MemoryRegion {
    /// Creates a new memory region descriptor.
    #[must_use]
    pub fn new(
        id: u64,
        offset: usize,
        size: usize,
        location: MemoryLocation,
        protection: MemoryProtection,
    ) -> Self {
        Self {
            id,
            offset,
            size,
            location,
            protection,
        }
    }

    /// Returns the region ID.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the byte offset.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the size in bytes.
    #[must_use]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns the memory location.
    #[must_use]
    pub fn location(&self) -> MemoryLocation {
        self.location
    }

    /// Returns true if the given range falls within this region.
    #[must_use]
    pub fn contains(&self, offset: usize, size: usize) -> bool {
        offset >= self.offset && offset + size <= self.offset + self.size
    }
}

/// Central memory manager that tracks all allocations.
#[derive(Debug)]
pub struct MemoryManager {
    host_pool: MemoryPool,
    arena: ArenaAllocator,
    total_host_allocated: AtomicUsize,
    peak_host_allocated: AtomicUsize,
    region_counter: AtomicUsize,
    regions: Mutex<Vec<MemoryRegion>>,
}

impl MemoryManager {
    /// Creates a new memory manager with default pools.
    #[must_use]
    pub fn new() -> Self {
        Self {
            host_pool: MemoryPool::new(1024 * 1024, 1024), // 1MB blocks, 1024 of them
            arena: ArenaAllocator::new(64 * 1024 * 1024),  // 64MB arena
            total_host_allocated: AtomicUsize::new(0),
            peak_host_allocated: AtomicUsize::new(0),
            region_counter: AtomicUsize::new(0),
            regions: Mutex::new(Vec::new()),
        }
    }

    /// Allocates host memory of the given size.
    pub fn alloc_host(&self, size: usize) -> NeuralResult<MemoryBlock> {
        let block = MemoryBlock::new(size, MemoryLocation::Host)?;
        let prev = self.total_host_allocated.fetch_add(size, Ordering::Relaxed);
        let new_total = prev + size;
        self.peak_host_allocated
            .fetch_max(new_total, Ordering::Relaxed);
        Ok(block)
    }

    /// Allocates from the arena (fast, bulk-free).
    pub fn alloc_arena(&self, size: usize) -> NeuralResult<usize> {
        self.arena.allocate(size)
    }

    /// Resets the arena allocator.
    pub fn reset_arena(&self) {
        self.arena.reset();
    }

    /// Registers a memory region for tracking.
    #[must_use]
    pub fn register_region(
        &self,
        offset: usize,
        size: usize,
        location: MemoryLocation,
        protection: MemoryProtection,
    ) -> MemoryRegion {
        let id = self.region_counter.fetch_add(1, Ordering::Relaxed) as u64;
        let region = MemoryRegion::new(id, offset, size, location, protection);
        self.regions.lock().push(region.clone());
        region
    }

    /// Returns total host memory currently allocated.
    #[must_use]
    pub fn total_host_allocated(&self) -> usize {
        self.total_host_allocated.load(Ordering::Relaxed)
    }

    /// Returns peak host memory allocated.
    #[must_use]
    pub fn peak_host_allocated(&self) -> usize {
        self.peak_host_allocated.load(Ordering::Relaxed)
    }

    /// Returns arena statistics.
    #[must_use]
    pub fn arena_total_allocated(&self) -> usize {
        self.arena.total_allocated()
    }

    /// Returns the host memory pool.
    #[must_use]
    pub fn host_pool(&self) -> &MemoryPool {
        &self.host_pool
    }

    /// Returns all registered memory regions.
    #[must_use]
    pub fn regions(&self) -> Vec<MemoryRegion> {
        self.regions.lock().clone()
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_block_alloc() {
        let block = MemoryBlock::new(1024, MemoryLocation::Host).unwrap();
        assert_eq!(block.size(), 1024);
        assert_eq!(block.location(), MemoryLocation::Host);
        assert_eq!(block.ref_count(), 1);
    }

    #[test]
    fn memory_block_refcount() {
        let block = MemoryBlock::new(256, MemoryLocation::Host).unwrap();
        block.increment_refs();
        assert_eq!(block.ref_count(), 2);
        assert!(!block.decrement_refs());
        assert!(block.decrement_refs());
    }

    #[test]
    fn arena_allocator() {
        let arena = ArenaAllocator::new(1024);
        let off1 = arena.allocate(100).unwrap();
        assert_eq!(off1, 0);
        let off2 = arena.allocate(200).unwrap();
        assert_eq!(off2, 100);
        assert_eq!(arena.total_allocated(), 300);
    }

    #[test]
    fn arena_reset() {
        let arena = ArenaAllocator::new(1024);
        let _ = arena.allocate(500).unwrap();
        arena.reset();
        let off = arena.allocate(100).unwrap();
        assert_eq!(off, 0);
    }

    #[test]
    fn memory_pool() {
        let pool = MemoryPool::new(256, 4);
        assert_eq!(pool.free_count(), 4);
        let off1 = pool.allocate().unwrap();
        assert_eq!(off1, 0);
        let off2 = pool.allocate().unwrap();
        assert_eq!(off2, 256);
        assert_eq!(pool.free_count(), 2);
        pool.free(off1);
        assert_eq!(pool.free_count(), 3);
    }

    #[test]
    fn memory_manager() {
        let mgr = MemoryManager::new();
        let block = mgr.alloc_host(512).unwrap();
        assert_eq!(block.size(), 512);
        assert_eq!(mgr.total_host_allocated(), 512);
        drop(block);
    }

    #[test]
    fn memory_region() {
        let mgr = MemoryManager::new();
        let region = mgr.register_region(
            0,
            4096,
            MemoryLocation::Host,
            MemoryProtection::ReadWrite,
        );
        assert_eq!(region.size(), 4096);
        assert!(region.contains(0, 4096));
        assert!(!region.contains(4096, 1));
    }
}
