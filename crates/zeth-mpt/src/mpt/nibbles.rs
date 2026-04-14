// Copyright 2025 RISC Zero, Inc.
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

//! A zero-cost abstraction for handling nibbles (4-bit values).
//!
//! This module provides `NibbleSlice`, a `Copy` wrapper around `Nibbles` that
//! offers efficient operations for working with nibble data.

use alloy_trie::Nibbles;
use core::fmt;

/// A slice of nibbles backed by an owned `Nibbles` value.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct NibbleSlice(Nibbles);

impl fmt::Debug for NibbleSlice {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl From<&Nibbles> for NibbleSlice {
    #[inline]
    fn from(nibbles: &Nibbles) -> Self {
        Self(*nibbles)
    }
}

impl From<Nibbles> for NibbleSlice {
    #[inline]
    fn from(nibbles: Nibbles) -> Self {
        Self(nibbles)
    }
}

impl From<NibbleSlice> for Nibbles {
    /// Converts a `NibbleSlice` back into a `Nibbles`.
    #[inline]
    fn from(slice: NibbleSlice) -> Self {
        slice.0
    }
}

#[allow(dead_code)]
impl NibbleSlice {
    #[inline]
    pub(super) const fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub(super) const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub(super) fn as_nibbles(&self) -> &Nibbles {
        &self.0
    }

    #[inline]
    pub(super) fn join(&self, other: impl Into<Self>) -> Nibbles {
        self.0.join(&other.into().0)
    }

    #[inline]
    pub(super) fn split_first(&self) -> Option<(u8, Self)> {
        if self.0.is_empty() {
            None
        } else {
            let nib = self.0.get_unchecked(0);
            Some((nib, Self(self.0.slice(1..))))
        }
    }

    #[inline]
    pub(super) fn strip_prefix(&self, prefix: &Nibbles) -> Option<Self> {
        if self.0.starts_with(prefix) {
            Some(Self(self.0.slice(prefix.len()..)))
        } else {
            None
        }
    }

    #[inline]
    pub(super) fn strip_suffix(&self, suffix: &Nibbles) -> Option<Self> {
        if self.0.ends_with(suffix) {
            Some(Self(self.0.slice(..self.0.len() - suffix.len())))
        } else {
            None
        }
    }

    /// Splits `self` and `other` at the first nibble that differs.
    #[inline]
    pub(super) fn split_common_prefix(&self, other: impl Into<Self>) -> (Self, Self, Self) {
        let other = other.into().0;
        let mid = self.0.common_prefix_length(&other);
        (Self(self.0.slice(..mid)), Self(self.0.slice(mid..)), Self(other.slice(mid..)))
    }
}
