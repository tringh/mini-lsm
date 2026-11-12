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

use crate::key::{Key, KeySlice};
use anyhow::Result;
use nom::combinator::Peek;
use std::borrow::Borrow;
use std::cmp::{self};
use std::collections::BinaryHeap;
use std::collections::binary_heap::PeekMut;
use std::iter::Repeat;
use std::mem;

use super::StorageIterator;

struct HeapWrapper<I: StorageIterator>(pub usize, pub Box<I>);

impl<I: StorageIterator> PartialEq for HeapWrapper<I> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
}

impl<I: StorageIterator> Eq for HeapWrapper<I> {}

impl<I: StorageIterator> PartialOrd for HeapWrapper<I> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<I: StorageIterator> Ord for HeapWrapper<I> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.1
            .key()
            .cmp(&other.1.key())
            .then(self.0.cmp(&other.0))
            .reverse()
    }
}

/// Merge multiple iterators of the same type. If the same key occurs multiple times in some
/// iterators, prefer the one with smaller index.
pub struct MergeIterator<I: StorageIterator> {
    iters: BinaryHeap<HeapWrapper<I>>,
    current: Option<HeapWrapper<I>>,
}

impl<I: StorageIterator> MergeIterator<I> {
    pub fn create(iters: Vec<Box<I>>) -> Self {
        let mut heap: BinaryHeap<HeapWrapper<I>> = BinaryHeap::new();
        iters
            .into_iter()
            .enumerate()
            .filter(|(_, iter)| iter.is_valid())
            .map(|(index, iter)| HeapWrapper(index, iter))
            .for_each(|wrapper| heap.push(wrapper));
        let current = heap.pop();
        MergeIterator {
            iters: heap,
            current,
        }
    }
}

impl<I: 'static + for<'a> StorageIterator<KeyType<'a> = KeySlice<'a>>> StorageIterator
    for MergeIterator<I>
{
    type KeyType<'a> = KeySlice<'a>;

    fn key(&self) -> KeySlice<'_> {
        let Some(iter) = &self.current else {
            return KeySlice::from_slice(&[]);
        };
        iter.1.key()
    }

    fn value(&self) -> &[u8] {
        let Some(iter) = &self.current else {
            return &[];
        };
        iter.1.value()
    }

    fn is_valid(&self) -> bool {
        let Some(iter) = &self.current else {
            return false;
        };
        iter.1.is_valid()
    }

    fn next(&mut self) -> Result<()> {
        let Some(cur) = self.current.as_mut() else {
            return Ok(());
        };

        while let Some(mut top) = self.iters.peek_mut()
            && top.1.key().eq(&cur.1.key())
        {
            match top.1.next() {
                Ok(()) => {
                    if !top.1.is_valid() {
                        PeekMut::pop(top);
                    }
                }
                Err(e) => {
                    PeekMut::pop(top);
                    return Err(e);
                }
            }
        }

        cur.1.next()?;

        if let Some(mut top) = self.iters.peek_mut()
            && (!cur.1.is_valid() || top.cmp(cur).is_gt())
        {
            if cur.1.is_valid() {
                mem::swap(&mut *top, cur);
            } else {
                self.current = Some(PeekMut::pop(top));
            }
        }
        Ok(())
    }

    // fn next(&mut self) -> Result<()> {
    //     let consumed_key = self.key().to_key_vec();
    //
    //     while let Some(mut top) = self.iters.peek_mut()
    //         && top.1.key().eq(&consumed_key.as_key_slice())
    //     {
    //         match top.1.next() {
    //             Ok(()) => {
    //                 if !top.1.is_valid() {
    //                     PeekMut::pop(top);
    //                 }
    //             }
    //             Err(e) => {
    //                 PeekMut::pop(top);
    //                 return Err(e);
    //             }
    //         }
    //     }
    //     Ok(())
    // }
}
