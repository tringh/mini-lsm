// Copyright (c) 2022-2026 Alex Chi Z
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use std::path::Path;
use std::sync::Arc;

use super::{BlockMeta, FileObject, SsTable};
use crate::key::KeyBytes;
use crate::{block::BlockBuilder, key::KeySlice, lsm_storage::BlockCache};
use anyhow::Result;
use bytes::{Bytes, BytesMut};

/// Builds an SSTable from key-value pairs.
pub struct SsTableBuilder {
    builder: BlockBuilder,
    first_key: Vec<u8>,
    last_key: Vec<u8>,
    data: Vec<u8>,
    pub(crate) meta: Vec<BlockMeta>,
    block_size: usize,
}

impl SsTableBuilder {
    /// Create a builder based on target block size.
    pub fn new(block_size: usize) -> Self {
        Self {
            builder: BlockBuilder::new(block_size),
            first_key: Vec::new(),
            last_key: Vec::new(),
            data: Vec::new(),
            meta: Vec::new(),
            block_size,
        }
    }

    /// Adds a key-value pair to SSTable.
    ///
    /// Note: You should split a new block when the current block is full.(`std::mem::replace` may
    /// be helpful here)
    pub fn add(&mut self, key: KeySlice, value: &[u8]) {
        if self.builder.is_empty() && self.meta.is_empty() {
            self.first_key = key.raw_ref().to_vec();
        }

        if self.builder.add(key, value) {
            self.last_key = key.raw_ref().to_vec();
            return;
        }

        self.advance_to_next_block();
        self.add(key, value);
    }

    fn advance_to_next_block(&mut self) {
        let meta = BlockMeta {
            offset: self.data.len(),
            first_key: self.builder.first_key().into_key_bytes(),
            last_key: KeyBytes::from_bytes(Bytes::copy_from_slice(&self.last_key)),
        };

        self.meta.push(meta);

        let builder = std::mem::replace(&mut self.builder, BlockBuilder::new(self.block_size));
        let block_data = builder.build().encode();
        self.data.extend_from_slice(&block_data)
    }

    /// Get the estimated size of the SSTable.
    ///
    /// Since the data blocks contain much more data than meta blocks, just return the size of data
    /// blocks here.
    pub fn estimated_size(&self) -> usize {
        self.data.len()
    }

    /// Builds the SSTable and writes it to the given path. Use the `FileObject` structure to manipulate the disk objects.
    pub fn build(
        #[allow(unused_mut)] mut self,
        id: usize,
        block_cache: Option<Arc<BlockCache>>,
        path: impl AsRef<Path>,
    ) -> Result<SsTable> {
        self.advance_to_next_block();

        let mut metadata = Vec::new();
        BlockMeta::encode_block_meta(self.meta.as_slice(), &mut metadata);
        let block_meta_offset = self.data.len();

        let mut data: Vec<u8> = Vec::with_capacity(self.data.len() + self.meta.len() + 4);
        data.extend(self.data);
        data.extend(metadata);
        data.extend((block_meta_offset as u32).to_le_bytes());

        let table = SsTable {
            file: FileObject::create(path.as_ref(), data)?,
            block_meta: self.meta,
            block_meta_offset,
            id,
            block_cache,
            first_key: KeyBytes::from_bytes(Bytes::from(self.first_key)),
            last_key: KeyBytes::from_bytes(Bytes::from(self.last_key)),
            bloom: None,
            max_ts: u64::MAX,
        };
        Ok(table)
    }

    #[cfg(test)]
    pub(crate) fn build_for_test(self, path: impl AsRef<Path>) -> Result<SsTable> {
        self.build(0, None, path)
    }
}
