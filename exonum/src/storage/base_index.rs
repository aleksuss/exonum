// Copyright 2017 The Exonum Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! An implementation of base index with most common features.
use std::borrow::Cow;
use std::marker::PhantomData;

use super::{StorageKey, StorageValue, Snapshot, Fork, Iter};

/// Basic struct for all indices that implements common features.
///
/// This structure is not intended for direct use, rather it is the basis for building other types
/// of indices.
///
/// `BaseIndex` requires that the keys implement the [`StorageKey`] trait and the values implement
/// [`StorageValue`] trait. However, this structure is not bound to specific types and allows the
/// use of *any* types as keys or values.
/// [`StorageKey`]: ../trait.StorageKey.html
/// [`StorageValue`]: ../trait.StorageValue.html
#[derive(Debug)]
pub struct BaseIndex<T> {
    name_idx: usize,
    prefix: Option<Vec<u8>>,
    view: T,
}

/// An iterator over the entries of a `BaseIndex`.
///
/// This struct is created by the [`iter`] or
/// [`iter_from`] methods on [`BaseIndex`]. See its documentation for more.
///
/// [`iter`]: struct.BaseIndex.html#method.iter
/// [`iter_from`]: struct.BaseIndex.html#method.iter_from
/// [`BaseIndex`]: struct.BaseIndex.html
pub struct BaseIndexIter<'a, K, V> {
    base_iter: Iter<'a>,
    base_prefix_len: usize,
    prefix: Vec<u8>,
    ended: bool,
    _k: PhantomData<K>,
    _v: PhantomData<V>,
}

impl<T> BaseIndex<T>
where
    T: AsRef<Snapshot>,
{
    /// Creates a new index representation based on the common prefix of its keys and storage view.
    ///
    /// Storage view can be specified as [`&Snapshot`] or [`&mut Fork`]. In the first case only
    /// immutable methods are available. In the second case both immutable and mutable methods are
    /// available.
    /// [`&Snapshot`]: ../trait.Snapshot.html
    /// [`&mut Fork`]: ../struct.Fork.html
    pub fn new(name: &str, view: T) -> Self {
        BaseIndex {
            name_idx: view.as_ref().get_index(name),
            prefix: None,
            view,
        }
    }

    /// Creates a new index with prefix
    pub fn with_prefix(name: &str, prefix: Vec<u8>, view: T) -> Self {
        BaseIndex {
            name_idx: view.as_ref().get_index(name),
            prefix: Some(prefix),
            view,
        }
    }
}

impl<T> BaseIndex<T> {
    fn prefixed_key<K: StorageKey>(&self, key: &K) -> Vec<u8> {
        match self.prefix {
            Some(ref prefix) => {
                let mut v = vec![0; prefix.len() + key.size()];
                v[..prefix.len()].copy_from_slice(&prefix);
                key.write(&mut v[prefix.len()..]);
                v
            }
            None => {
                let mut v = vec![0; key.size()];
                key.write(&mut v);
                v
            }
        }
    }
}

impl<T> BaseIndex<T>
where
    T: AsRef<Snapshot>,
{
    /// Returns a value of *any* type corresponding to the key of *any* type.
    pub fn get<K, V>(&self, key: &K) -> Option<V>
    where
        K: StorageKey,
        V: StorageValue,
    {
        self.view
            .as_ref()
            .get(self.name_idx, &self.prefixed_key(key))
            .map(|v| StorageValue::from_bytes(Cow::Owned(v)))
    }

    /// Returns `true` if the index contains a value of *any* type for the specified key of
    /// *any* type.
    pub fn contains<K>(&self, key: &K) -> bool
    where
        K: StorageKey,
    {
        self.view.as_ref().contains(
            self.name_idx,
            &self.prefixed_key(key),
        )
    }

    /// Returns an iterator over the entries of the index in ascending order. The iterator element
    /// type is *any* key-value pair. An argument `subprefix` allows to specify a subset of keys
    /// for iteration.
    pub fn iter<P, K, V>(&self, subprefix: &P) -> BaseIndexIter<K, V>
    where
        P: StorageKey,
        K: StorageKey,
        V: StorageValue,
    {
        let iter_prefix = self.prefixed_key(subprefix);
        BaseIndexIter {
            base_iter: self.view.as_ref().iter(self.name_idx, &iter_prefix),
            base_prefix_len: self.prefix.as_ref().map_or(0, |p| p.len()),
            prefix: iter_prefix,
            ended: false,
            _k: PhantomData,
            _v: PhantomData,
        }
    }

    /// Returns an iterator over the entries of the index in ascending order starting from the
    /// specified key. The iterator element type is *any* key-value pair. An argument `subprefix`
    /// allows to specify a subset of iteration.
    pub fn iter_from<P, F, K, V>(&self, subprefix: &P, from: &F) -> BaseIndexIter<K, V>
    where
        P: StorageKey,
        F: StorageKey,
        K: StorageKey,
        V: StorageValue,
    {
        let iter_prefix = self.prefixed_key(subprefix);
        let iter_from = self.prefixed_key(from);
        BaseIndexIter {
            base_iter: self.view.as_ref().iter(self.name_idx, &iter_from),
            base_prefix_len: self.prefix.as_ref().map_or(0, |p| p.len()),
            prefix: iter_prefix,
            ended: false,
            _k: PhantomData,
            _v: PhantomData,
        }
    }
}

impl<'a> BaseIndex<&'a mut Fork> {
    /// Inserts the key-value pair into the index. Both key and value may be of *any* types.
    pub fn put<K, V>(&mut self, key: &K, value: V)
    where
        K: StorageKey,
        V: StorageValue,
    {
        let key = self.prefixed_key(key);
        self.view.put(self.name_idx, key, value.into_bytes());
    }

    /// Removes the key of *any* type from the index.
    pub fn remove<K>(&mut self, key: &K)
    where
        K: StorageKey,
    {
        let key = self.prefixed_key(key);
        self.view.remove(self.name_idx, key);
    }

    /// Clears the index, removing all entries.
    ///
    /// # Notes
    ///
    /// Currently this method is not optimized to delete large set of data. During the execution of
    /// this method the amount of allocated memory is linearly dependent on the number of elements
    /// in the index.
    pub fn clear(&mut self) {
        if let Some(ref prefix) = self.prefix {
            self.view.remove_by_prefix(self.name_idx, prefix);
        }
    }
}

impl<'a, K, V> Iterator for BaseIndexIter<'a, K, V>
where
    K: StorageKey,
    V: StorageValue,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        if let Some((k, v)) = self.base_iter.next() {
            if k.starts_with(&self.prefix) {
                return Some((
                    K::read(&k[self.base_prefix_len..]),
                    V::from_bytes(Cow::Borrowed(v)),
                ));
            }
        }
        self.ended = true;
        None
    }
}

impl<'a, K, V> ::std::fmt::Debug for BaseIndexIter<'a, K, V> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "BaseIndexIter(..)")
    }
}
