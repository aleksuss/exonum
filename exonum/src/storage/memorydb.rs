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

//! An implementation of `MemoryDB` database.
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::clone::Clone;
use std::collections::btree_map::{BTreeMap, Range};
use std::collections::HashMap;
use std::iter::Peekable;

use super::{Database, Snapshot, Patch, Change, Iterator, Iter, Result};

const DEFAULT_TABLE: &'static str = "default";

type DB = Arc<RwLock<HashMap<String, BTreeMap<Vec<u8>, Vec<u8>>>>>;

/// Database implementation that stores all the data in memory.
///
/// It's mainly used for testing and not designed to be efficient.
#[derive(Default, Clone, Debug)]
pub struct MemoryDB {
    map: DB,
}

/// An iterator over the entries of a `MemoryDB`.
struct MemoryDBIter<'a> {
    iter: Peekable<Range<'a, Vec<u8>, Vec<u8>>>,
    _guard: RwLockReadGuard<'a, HashMap<String, BTreeMap<Vec<u8>, Vec<u8>>>>,
}

impl MemoryDB {
    /// Creates a new, empty database.
    pub fn new() -> MemoryDB {
        let mut tables = HashMap::new();
        debug_assert_eq!(
            tables.insert(DEFAULT_TABLE.to_string(), BTreeMap::new()),
            None
        );
        MemoryDB { map: Arc::new(RwLock::new(tables)) }
    }
}

impl Database for MemoryDB {
    fn clone(&self) -> Box<Database> {
        Box::new(Clone::clone(self))
    }

    fn snapshot(&self) -> Box<Snapshot> {
        Box::new(MemoryDB {
            map: Arc::new(RwLock::new(self.map.read().unwrap().clone())),
        })
    }

    fn merge(&mut self, patch: Patch) -> Result<()> {
        for (cf_name, idx) in patch.mapping.borrow().iter() {
            let mut guard = self.map.write().unwrap();
            if !guard.contains_key(cf_name) {
                guard.insert(cf_name.clone(), BTreeMap::new());
            }
            let table = guard.get_mut(cf_name).unwrap();
            for (key, change) in patch.changes.get(*idx).cloned().unwrap() {
                match change {
                    Change::Put(ref value) => {
                        table.insert(key, value.to_vec());
                    }
                    Change::Delete => {
                        table.remove(&key);
                    }
                }
            }
        }
        Ok(())
    }
}

impl Snapshot for MemoryDB {
    fn get(&self, _name_idx: usize, key: &[u8]) -> Option<Vec<u8>> {
        let cf_name = "ddddd";
        self.map.read().unwrap().get(cf_name).and_then(|table| {
            table.get(key).cloned()
        })
    }

    fn contains(&self, _name_idx: usize, key: &[u8]) -> bool {
        let cf_name = "ddddd";
        self.map.read().unwrap().get(cf_name).map_or(
            false,
            |table| {
                table.contains_key(key)
            },
        )
    }

    fn iter<'a>(&'a self, _name_idx: usize, from: &[u8]) -> Iter<'a> {
        use std::collections::Bound::{Included, Unbounded};
        use std::mem::transmute;
        let guard = self.map.read().unwrap();
        let cf_name = "ddddd";

        Box::new(MemoryDBIter {
            iter: unsafe {
                transmute(match guard.get(cf_name) {
                    Some(table) => {
                        table
                            .range::<[u8], _>((Included(from), Unbounded))
                            .peekable()
                    }
                    None => {
                        guard
                            .get(DEFAULT_TABLE)
                            .unwrap()
                            .range::<[u8], _>((Unbounded, Unbounded))
                            .peekable()
                    }
                })
            },
            _guard: guard,
        })
    }

    fn get_index(&self, _name: &str) -> usize {
        0
    }
}

impl<'a> Iterator for MemoryDBIter<'a> {
    fn next(&mut self) -> Option<(&[u8], &[u8])> {
        self.iter.next().map(|(k, v)| (k.as_slice(), v.as_slice()))
    }

    fn peek(&mut self) -> Option<(&[u8], &[u8])> {
        self.iter.peek().map(|&(k, v)| (k.as_slice(), v.as_slice()))
    }
}

#[test]
fn test_memorydb_snapshot() {
    let mut db = MemoryDB::new();

    {
        let mut fork = db.fork();
        let name_idx = fork.get_index(DEFAULT_TABLE);
        fork.put(name_idx, vec![1, 2, 3], vec![123]);
        let _ = db.merge(fork.into_patch());
    }

    let snapshot = db.snapshot();
    let name_idx = snapshot.get_index(DEFAULT_TABLE);
    assert!(snapshot.contains(name_idx, vec![1, 2, 3].as_slice()));

    {
        let mut fork = db.fork();
        let name_idx = fork.get_index(DEFAULT_TABLE);
        fork.put(name_idx, vec![2, 3, 4], vec![234]);
        let _ = db.merge(fork.into_patch());
    }

    assert!(!snapshot.contains(name_idx, vec![2, 3, 4].as_slice()));

    {
        let db_clone = Clone::clone(&db);
        let snap_clone = db_clone.snapshot();
        let name_idx = snap_clone.get_index(DEFAULT_TABLE);
        assert!(snap_clone.contains(name_idx, vec![2, 3, 4].as_slice()));
    }

    let snapshot = db.snapshot();
    let name_idx = snapshot.get_index(DEFAULT_TABLE);
    assert!(snapshot.contains(name_idx, vec![2, 3, 4].as_slice()));
}
