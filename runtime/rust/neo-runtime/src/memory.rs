use serde::{Deserialize, Serialize};

use neo_core::error::{NeoError, NeoResult};

/// Memory protection flags for a memory region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryProtection {
    Read,
    Write,
    Execute,
    ReadWrite,
    ReadExecute,
    ReadWriteExecute,
}

impl std::fmt::Display for MemoryProtection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryProtection::Read => write!(f, "r--"),
            MemoryProtection::Write => write!(f, "-w-"),
            MemoryProtection::Execute => write!(f, "--x"),
            MemoryProtection::ReadWrite => write!(f, "rw-"),
            MemoryProtection::ReadExecute => write!(f, "r-x"),
            MemoryProtection::ReadWriteExecute => write!(f, "rwx"),
        }
    }
}

/// A contiguous block of managed memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRegion {
    pub base_address: usize,
    pub size: usize,
    pub protection: MemoryProtection,
}

/// Manages allocation and deallocation of memory regions.
pub struct MemoryManager {
    total_bytes: usize,
    used_bytes: usize,
    next_address: usize,
    regions: Vec<MemoryRegion>,
}

impl MemoryManager {
    /// Create a new memory manager with the given total capacity.
    pub fn new(total_bytes: usize) -> Self {
        Self {
            total_bytes,
            used_bytes: 0,
            next_address: 0x10000,
            regions: Vec::new(),
        }
    }

    /// Allocate a region of the given size with default read-write protection.
    pub fn allocate(&mut self, size: usize) -> NeoResult<MemoryRegion> {
        if size == 0 {
            return Err(NeoError::InvalidInput(
                "allocation size must be greater than zero".to_string(),
            ));
        }

        if self.used_bytes + size > self.total_bytes {
            return Err(NeoError::ResourceExhausted(format!(
                "insufficient memory: requested {} bytes, {} available",
                size,
                self.total_bytes - self.used_bytes
            )));
        }

        let region = MemoryRegion {
            base_address: self.next_address,
            size,
            protection: MemoryProtection::ReadWrite,
        };

        self.next_address += size;
        self.used_bytes += size;
        self.regions.push(region.clone());

        Ok(region)
    }

    /// Deallocate a previously allocated memory region.
    pub fn deallocate(&mut self, region: MemoryRegion) {
        if let Some(pos) = self.regions.iter().position(|r| r.base_address == region.base_address) {
            self.regions.remove(pos);
            self.used_bytes -= region.size;
        }
    }

    /// Returns the number of bytes currently in use.
    pub fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    /// Returns the number of bytes currently free.
    pub fn free_bytes(&self) -> usize {
        self.total_bytes - self.used_bytes
    }

    /// Returns the total capacity in bytes.
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Returns a slice of all currently allocated regions.
    pub fn regions(&self) -> &[MemoryRegion] {
        &self.regions
    }
}
