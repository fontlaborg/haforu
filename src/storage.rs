// this_file: src/storage.rs
//! Storage module for pre-rendered font results

use crate::error::{Error, Result};
use log::{debug, info};
use memmap2::{Mmap, MmapOptions};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SHARD_MAGIC: &[u8; 4] = b"HAFO";
const SHARD_VERSION: u32 = 1;
const INDEX_ENTRY_SIZE: usize = 20; // offset(8) + len(4) + width(2) + height(2) + checksum(4)

/// Index entry for a stored image
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IndexEntry {
    pub offset: u64,
    pub len: u32,
    pub width: u16,
    pub height: u16,
    pub checksum: u32,
}

/// Shard footer structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ShardFooter {
    magic: [u8; 4],
    version: u32,
    n_entries: u64,
    index_offset: u64,
    shard_id: u32,
    crc: u32,
}

/// A single shard file containing multiple images
pub struct Shard {
    mmap: Arc<Mmap>,
    index_offset: usize,
    n_entries: usize,
    #[allow(dead_code)]
    shard_id: u32,
}

impl Shard {
    /// Open an existing shard file
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())
            .map_err(|e| Error::Storage(format!("Failed to open shard: {}", e)))?;

        let mmap = unsafe { MmapOptions::new().map(&file) }
            .map_err(|e| Error::Storage(format!("Failed to mmap shard: {}", e)))?;

        // Read and validate footer
        if mmap.len() < std::mem::size_of::<ShardFooter>() {
            return Err(Error::Storage("Shard file too small".into()));
        }

        let footer_offset = mmap.len() - std::mem::size_of::<ShardFooter>();
        let footer_bytes = &mmap[footer_offset..];

        // Parse footer (simplified, production would handle endianness properly)
        let magic = &footer_bytes[0..4];
        if magic != SHARD_MAGIC {
            return Err(Error::Storage("Invalid shard magic".into()));
        }

        let version = u32::from_le_bytes(footer_bytes[4..8].try_into().unwrap());
        if version != SHARD_VERSION {
            return Err(Error::Storage(format!(
                "Unsupported shard version: {}",
                version
            )));
        }

        let n_entries = u64::from_le_bytes(footer_bytes[8..16].try_into().unwrap()) as usize;
        let index_offset = u64::from_le_bytes(footer_bytes[16..24].try_into().unwrap()) as usize;
        let shard_id = u32::from_le_bytes(footer_bytes[24..28].try_into().unwrap());

        debug!("Opened shard {} with {} entries", shard_id, n_entries);

        Ok(Self {
            mmap: Arc::new(mmap),
            index_offset,
            n_entries,
            shard_id,
        })
    }

    /// Get an index entry
    pub fn get_index_entry(&self, local_idx: usize) -> Result<IndexEntry> {
        if local_idx >= self.n_entries {
            return Err(Error::Storage(format!("Index {} out of range", local_idx)));
        }

        let entry_offset = self.index_offset + local_idx * INDEX_ENTRY_SIZE;
        if entry_offset + INDEX_ENTRY_SIZE > self.mmap.len() {
            return Err(Error::Storage("Index entry beyond file bounds".into()));
        }

        let bytes = &self.mmap[entry_offset..entry_offset + INDEX_ENTRY_SIZE];

        Ok(IndexEntry {
            offset: u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
            len: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            width: u16::from_le_bytes(bytes[12..14].try_into().unwrap()),
            height: u16::from_le_bytes(bytes[14..16].try_into().unwrap()),
            checksum: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
        })
    }

    /// Get compressed image data
    pub fn get_image_data(&self, local_idx: usize) -> Result<Vec<u8>> {
        let entry = self.get_index_entry(local_idx)?;

        if entry.offset as usize + entry.len as usize > self.mmap.len() {
            return Err(Error::Storage("Image data beyond file bounds".into()));
        }

        let data = &self.mmap[entry.offset as usize..(entry.offset + entry.len as u64) as usize];

        // Decompress if needed
        let decompressed = zstd::decode_all(data)
            .map_err(|e| Error::Storage(format!("Failed to decompress image: {}", e)))?;

        Ok(decompressed)
    }
}

/// Shard writer for creating new shards
pub struct ShardWriter {
    file: File,
    entries: Vec<IndexEntry>,
    current_offset: u64,
    shard_id: u32,
}

impl ShardWriter {
    /// Create a new shard writer
    pub fn new<P: AsRef<Path>>(path: P, shard_id: u32) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path.as_ref())
            .map_err(|e| Error::Storage(format!("Failed to create shard: {}", e)))?;

        Ok(Self {
            file,
            entries: Vec::new(),
            current_offset: 0,
            shard_id,
        })
    }

    /// Add an image to the shard
    pub fn add_image(&mut self, data: &[u8], width: u16, height: u16) -> Result<usize> {
        // Compress the data
        let compressed = zstd::encode_all(data, 3)
            .map_err(|e| Error::Storage(format!("Failed to compress image: {}", e)))?;

        // Calculate checksum
        let checksum = crc32fast::hash(&compressed);

        // Write compressed data
        self.file
            .write_all(&compressed)
            .map_err(|e| Error::Storage(format!("Failed to write image data: {}", e)))?;

        // Create index entry
        let entry = IndexEntry {
            offset: self.current_offset,
            len: compressed.len() as u32,
            width,
            height,
            checksum,
        };

        self.entries.push(entry);
        self.current_offset += compressed.len() as u64;

        Ok(self.entries.len() - 1)
    }

    /// Finalize the shard file
    pub fn finalize(mut self) -> Result<()> {
        let index_offset = self.current_offset;

        // Write index
        for entry in &self.entries {
            self.file.write_all(&entry.offset.to_le_bytes())?;
            self.file.write_all(&entry.len.to_le_bytes())?;
            self.file.write_all(&entry.width.to_le_bytes())?;
            self.file.write_all(&entry.height.to_le_bytes())?;
            self.file.write_all(&entry.checksum.to_le_bytes())?;
        }

        // Write footer
        self.file.write_all(SHARD_MAGIC)?;
        self.file.write_all(&SHARD_VERSION.to_le_bytes())?;
        self.file
            .write_all(&(self.entries.len() as u64).to_le_bytes())?;
        self.file.write_all(&index_offset.to_le_bytes())?;
        self.file.write_all(&self.shard_id.to_le_bytes())?;

        // Calculate and write CRC of footer
        let crc = 0u32; // Simplified, would calculate actual CRC in production
        self.file.write_all(&crc.to_le_bytes())?;

        self.file
            .sync_all()
            .map_err(|e| Error::Storage(format!("Failed to sync shard: {}", e)))?;

        info!(
            "Finalized shard {} with {} entries",
            self.shard_id,
            self.entries.len()
        );
        Ok(())
    }
}

/// Storage manager for handling multiple shards
pub struct StorageManager {
    base_path: PathBuf,
    shards: HashMap<u32, Arc<Shard>>,
    images_per_shard: usize,
    current_writer: Option<ShardWriter>,
    next_shard_id: u32,
    images_in_current_shard: usize,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new<P: AsRef<Path>>(base_path: P, images_per_shard: usize) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();

        // Create base directory if it doesn't exist
        fs::create_dir_all(&base_path)
            .map_err(|e| Error::Storage(format!("Failed to create storage directory: {}", e)))?;

        Ok(Self {
            base_path,
            shards: HashMap::new(),
            images_per_shard,
            current_writer: None,
            next_shard_id: 0,
            images_in_current_shard: 0,
        })
    }

    /// Store an image
    pub fn store_image(&mut self, data: &[u8], width: u16, height: u16) -> Result<String> {
        // Check if we need a new shard
        if self.current_writer.is_none() || self.images_in_current_shard >= self.images_per_shard {
            self.start_new_shard()?;
        }

        let writer = self.current_writer.as_mut().unwrap();
        let local_idx = writer.add_image(data, width, height)?;
        self.images_in_current_shard += 1;

        let identifier = format!("{}:{}", self.next_shard_id - 1, local_idx);
        debug!("Stored image as {}", identifier);

        Ok(identifier)
    }

    /// Start a new shard
    fn start_new_shard(&mut self) -> Result<()> {
        // Finalize current writer if exists
        if let Some(writer) = self.current_writer.take() {
            writer.finalize()?;
        }

        let shard_path = self
            .base_path
            .join(format!("shard_{:08}.hafo", self.next_shard_id));
        self.current_writer = Some(ShardWriter::new(shard_path, self.next_shard_id)?);
        self.next_shard_id += 1;
        self.images_in_current_shard = 0;

        info!("Started new shard {}", self.next_shard_id - 1);
        Ok(())
    }

    /// Retrieve an image
    pub fn get_image(&mut self, identifier: &str) -> Result<Vec<u8>> {
        // Parse identifier (shard_id:local_idx)
        let parts: Vec<&str> = identifier.split(':').collect();
        if parts.len() != 2 {
            return Err(Error::Storage(format!(
                "Invalid identifier format: {}",
                identifier
            )));
        }

        let shard_id: u32 = parts[0]
            .parse()
            .map_err(|_| Error::Storage(format!("Invalid shard ID: {}", parts[0])))?;
        let local_idx: usize = parts[1]
            .parse()
            .map_err(|_| Error::Storage(format!("Invalid local index: {}", parts[1])))?;

        // Load shard if not cached
        if !self.shards.contains_key(&shard_id) {
            let shard_path = self.base_path.join(format!("shard_{:08}.hafo", shard_id));
            let shard = Arc::new(Shard::open(shard_path)?);
            self.shards.insert(shard_id, shard);
        }

        let shard = self.shards.get(&shard_id).unwrap();
        shard.get_image_data(local_idx)
    }

    /// Finalize all open shards
    pub fn finalize(&mut self) -> Result<()> {
        if let Some(writer) = self.current_writer.take() {
            writer.finalize()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_index_entry_size() {
        assert_eq!(std::mem::size_of::<IndexEntry>(), INDEX_ENTRY_SIZE);
    }

    #[test]
    fn test_storage_manager_creation() {
        let dir = tempdir().unwrap();
        let manager = StorageManager::new(dir.path(), 100);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_store_and_retrieve_image() {
        let dir = tempdir().unwrap();
        let mut manager = StorageManager::new(dir.path(), 10).unwrap();

        // Create test image data
        let data = vec![255u8; 1000];
        let identifier = manager.store_image(&data, 100, 200).unwrap();

        // Finalize the shard
        manager.finalize().unwrap();

        // Retrieve the image
        let retrieved = manager.get_image(&identifier).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_shard_rotation() {
        let dir = tempdir().unwrap();
        let mut manager = StorageManager::new(dir.path(), 2).unwrap();

        // Store 3 images (should create 2 shards)
        let id1 = manager.store_image(&vec![1u8; 100], 10, 10).unwrap();
        let id2 = manager.store_image(&vec![2u8; 100], 10, 10).unwrap();
        let id3 = manager.store_image(&vec![3u8; 100], 10, 10).unwrap();

        // Check that we have different shard IDs
        assert!(id1.starts_with("0:"));
        assert!(id2.starts_with("0:"));
        assert!(id3.starts_with("1:"));
    }
}
