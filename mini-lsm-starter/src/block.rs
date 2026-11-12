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

mod builder;
mod iterator;

pub use builder::BlockBuilder;
use bytes::{Buf, BufMut, Bytes, BytesMut};
pub use iterator::BlockIterator;
use std::io::Read;
use std::ops::Add;

pub(crate) const SIZEOF_U16: usize = std::mem::size_of::<u16>();

fn read_section_size_u16(data: &[u8], off: usize) -> usize {
    let bytes = data
        .get(off..off.add(SIZEOF_U16))
        .expect("Odd block format");
    u16::from_le_bytes(bytes.try_into().expect("Odd block format")) as usize
}

/// A block is the smallest unit of read and caching in LSM tree. It is a collection of sorted key-value pairs.
pub struct Block {
    pub(crate) data: Vec<u8>,
    pub(crate) offsets: Vec<u16>,
}

impl Block {
    /// Encode the internal data to the data layout illustrated in the course
    /// Note: You may want to recheck if any of the expected field is missing from your output
    pub fn encode(&self) -> Bytes {
        let mut result: Vec<u8> = Vec::new();
        result.extend(self.data.iter());
        // result.extend(self.offsets.iter()
        //     .flat_map(|e| e.to_le_bytes()));
        self.offsets
            .iter()
            .for_each(|e| result.put_u16_le(e.clone()));
        result.put_u16_le(self.offsets.len() as u16);
        Bytes::from(result)
    }

    /// Decode from the data layout, transform the input `data` to a single `Block`
    pub fn decode(data: &[u8]) -> Self {
        let num_entries = u16::from_le_bytes(data[data.len() - 2..].try_into().unwrap()) as usize;

        let offsets: Vec<u16> = data[data.len() - 2 - num_entries * 2..data.len() - 2]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();

        let data: Vec<u8> = data[0..data.len() - 2 - num_entries * 2]
            .iter()
            .map(|e| e.clone())
            .collect();

        Self { data, offsets }
    }
}
