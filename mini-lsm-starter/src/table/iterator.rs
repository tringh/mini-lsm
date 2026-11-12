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

use std::fmt::format;
use std::sync::Arc;

use super::SsTable;
use crate::{block::BlockIterator, iterators::StorageIterator, key::KeySlice};
use anyhow::{Result, anyhow};
use nom::character::complete::tab;

/// An iterator over the contents of an SSTable.
pub struct SsTableIterator {
    table: Arc<SsTable>,
    blk_iter: BlockIterator,
    blk_idx: usize,
}

impl SsTableIterator {
    /// Create a new iterator and seek to the first key-value pair in the first data block.
    pub fn create_and_seek_to_first(table: Arc<SsTable>) -> Result<Self> {
        let first_block = table.read_block_cached(0)?;
        Ok(Self {
            table,
            blk_iter: BlockIterator::create_and_seek_to_first(first_block),
            blk_idx: 0,
        })
    }

    /// Seek to the first key-value pair in the first data block.
    pub fn seek_to_first(&mut self) -> Result<()> {
        let first_block = self.table.read_block_cached(0)?;
        self.blk_iter = BlockIterator::create_and_seek_to_first(first_block);
        self.blk_idx = 0;
        Ok(())
    }

    /// Create a new iterator and seek to the first key-value pair which >= `key`.
    pub fn create_and_seek_to_key(table: Arc<SsTable>, key: KeySlice) -> Result<Self> {
        let Some((block_iter, block_idx)) = Self::get_iter_seek_to_key(table.clone(), key)? else {
            return Err(anyhow!("Cannot find appropriate key"));
        };

        Ok(Self {
            table,
            blk_iter: block_iter,
            blk_idx: block_idx,
        })
    }

    /// Seek to the first key-value pair which >= `key`.
    /// Note: You probably want to review the handout for detailed explanation when implementing
    /// this function.
    pub fn seek_to_key(&mut self, key: KeySlice) -> Result<()> {
        let Some((block_iter, block_idx)) = Self::get_iter_seek_to_key(self.table.clone(), key)?
        else {
            return Ok(());
        };
        self.blk_iter = block_iter;
        self.blk_idx = block_idx;
        Ok(())
    }

    fn get_iter_seek_to_key(
        table: Arc<SsTable>,
        key: KeySlice,
    ) -> Result<Option<(BlockIterator, usize)>> {
        let mut block_idx = table.find_block_idx(key);
        let last_key_vs_key = table.block_meta[block_idx]
            .last_key
            .cmp(&key.to_key_vec().into_key_bytes());
        if last_key_vs_key.is_lt() {
            block_idx = block_idx + 1;
            if block_idx == table.block_meta.len() {
                //return Err(anyhow!("Key out of range"));
                return Ok(None);
            };
        }
        let block = table.read_block_cached(block_idx)?;
        let block_iter = BlockIterator::create_and_seek_to_key(block, key);
        Ok(Some((block_iter, block_idx)))
    }

    fn is_last_iter(&self) -> bool {
        self.blk_idx == self.table.block_meta.len() - 1
    }
}

impl StorageIterator for SsTableIterator {
    type KeyType<'a> = KeySlice<'a>;

    /// Return the `key` that's held by the underlying block iterator.
    fn key(&self) -> KeySlice<'_> {
        self.blk_iter.key()
    }

    /// Return the `value` that's held by the underlying block iterator.
    fn value(&self) -> &[u8] {
        self.blk_iter.value()
    }

    /// Return whether the current block iterator is valid or not.
    fn is_valid(&self) -> bool {
        self.blk_iter.is_valid()
    }

    /// Move to the next `key` in the block.
    /// Note: You may want to check if the current block iterator is valid after the move.
    fn next(&mut self) -> Result<()> {
        if !self.blk_iter.is_valid() && self.is_last_iter() {
            return Ok(());
        }
        self.blk_iter.next();
        if !self.blk_iter.is_valid() && !self.is_last_iter() {
            let next_block = self.table.read_block(self.blk_idx + 1)?;
            let next_iter = BlockIterator::create_and_seek_to_first(next_block);
            self.blk_iter = next_iter;
            self.blk_idx += 1;
        }
        Ok(())
    }
}
