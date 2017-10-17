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

//! An implementation of `RocksDB` database.
use exonum_profiler::ProfilerSpan;
use rocksdb::DB as _RocksDB;
use rocksdb::{WriteBatch, DBRawIterator};
use rocksdb::Snapshot as _Snapshot;
use rocksdb::Error as _Error;
use rocksdb::utils::get_cf_names;

use std::mem;
use std::sync::Arc;
use std::path::Path;
use std::fmt;
use std::error;

pub use rocksdb::Options as RocksDBOptions;
pub use rocksdb::BlockBasedOptions as RocksBlockOptions;

use super::{Database, Iterator, Iter, Snapshot, Error, Patch, Change, Result};

impl From<_Error> for Error {
    fn from(err: _Error) -> Self {
        Error::new(error::Error::description(&err))
    }
}

/// Database implementation on the top of `RocksDB` backend.
#[derive(Clone)]
pub struct RocksDB {
    db: Arc<_RocksDB>,
}

/// A snapshot of a `RocksDB`.
pub struct RocksDBSnapshot {
    snapshot: _Snapshot<'static>,
    _db: Arc<_RocksDB>,
}

/// An iterator over the entries of a `RocksDB`.
struct RocksDBIterator {
    iter: DBRawIterator,
    key: Option<Vec<u8>>,
    value: Option<Vec<u8>>,
}

impl RocksDB {
    /// Open a database stored in the specified path with the specified options.
    pub fn open(path: &Path, options: RocksDBOptions) -> Result<RocksDB> {
        let db = {
            if let Ok(names) = get_cf_names(path) {
                let cf_names = names.iter().map(|name| name.as_str()).collect::<Vec<_>>();
                _RocksDB::open_cf(&options, path, cf_names.as_ref())?
            } else {
                _RocksDB::open(&options, path)?
            }
        };
        Ok(RocksDB { db: Arc::new(db) })
    }
}

impl Database for RocksDB {
    fn clone(&self) -> Box<Database> {
        Box::new(Clone::clone(self))
    }

    fn snapshot(&self) -> Box<Snapshot> {
        let _p = ProfilerSpan::new("RocksDB::snapshot");
        Box::new(RocksDBSnapshot {
            snapshot: unsafe { mem::transmute(self.db.snapshot()) },
            _db: Arc::clone(&self.db),
        })
    }

    fn merge(&mut self, patch: Patch) -> Result<()> {
        let _p = ProfilerSpan::new("RocksDB::merge");
        let mut batch = WriteBatch::default();
        for (cf_name, changes) in patch {
            let cf = match self.db.cf_handle(&cf_name) {
                Some(cf) => cf,
                None => {
                    self.db
                        .create_cf(&cf_name, &RocksDBOptions::default())
                        .unwrap()
                }
            };
            for (key, change) in changes {
                match change {
                    Change::Put(ref value) => batch.put_cf(cf, &key, value)?,
                    Change::Delete => batch.delete_cf(cf, &key)?,
                }
            }
        }
        self.db.write(batch).map_err(Into::into)
    }
}

impl Snapshot for RocksDBSnapshot {
    fn get(&self, cf_name: &str, key: &[u8]) -> Option<Vec<u8>> {
        let _p = ProfilerSpan::new("RocksDBSnapshot::get");
        if let Some(cf) = self._db.cf_handle(cf_name) {
            match self.snapshot.get_cf(cf, key) {
                Ok(value) => value.map(|v| v.to_vec()),
                Err(e) => panic!(e),
            }
        } else {
            None
        }
    }

    fn iter<'a>(&'a self, cf_name: &str, from: &[u8]) -> Iter<'a> {
        let _p = ProfilerSpan::new("RocksDBSnapshot::iter");
        let iter = match self._db.cf_handle(cf_name) {
            Some(cf) => {
                let mut iter = self.snapshot.raw_iterator_cf(cf).unwrap();
                iter.seek(from);
                iter
            }
            None => self.snapshot.raw_iterator(),
        };
        Box::new(RocksDBIterator {
            iter,
            key: None,
            value: None,
        })
    }
}

impl<'a> Iterator for RocksDBIterator {
    fn next(&mut self) -> Option<(&[u8], &[u8])> {
        let _p = ProfilerSpan::new("RocksDBIterator::next");
        let result = if self.iter.valid() {
            self.key = Some(unsafe { self.iter.key_inner().unwrap().to_vec() });
            self.value = Some(unsafe { self.iter.value_inner().unwrap().to_vec() });
            Some((
                self.key.as_ref().unwrap().as_ref(),
                self.value.as_ref().unwrap().as_ref(),
            ))
        } else {
            None
        };

        if result.is_some() {
            self.iter.next();
        }

        result
    }

    fn peek(&mut self) -> Option<(&[u8], &[u8])> {
        let _p = ProfilerSpan::new("RocksDBIterator::peek");
        if self.iter.valid() {
            self.key = Some(unsafe { self.iter.key_inner().unwrap().to_vec() });
            self.value = Some(unsafe { self.iter.value_inner().unwrap().to_vec() });
            Some((
                self.key.as_ref().unwrap().as_ref(),
                self.value.as_ref().unwrap().as_ref(),
            ))
        } else {
            None
        }
    }
}

impl fmt::Debug for RocksDB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RocksDB(..)")
    }
}

impl fmt::Debug for RocksDBSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RocksDBSnapshot(..)")
    }
}
