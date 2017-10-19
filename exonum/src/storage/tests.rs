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

use super::{Database, Snapshot, Fork};

const TABLE_NAME: &'static str = "table";

fn fork_iter<T: Database>(mut db: T) {
    let mut fork = db.fork();
    let name_idx = fork.get_index(TABLE_NAME);
    fork.put(name_idx, vec![10], vec![10]);
    fork.put(name_idx, vec![20], vec![20]);
    fork.put(name_idx, vec![30], vec![30]);

    assert!(fork.contains(name_idx, &[10]));

    db.merge(fork.into_patch()).unwrap();

    let mut fork = db.fork();
    let name_idx = fork.get_index(TABLE_NAME);
    assert!(fork.contains(name_idx, &[10]));

    fn assert_iter(fork: &Fork, from: u8, assumed: &[(u8, u8)]) {
        let mut values = Vec::new();
        let name_idx = fork.get_index(TABLE_NAME);
        let mut iter = fork.iter(name_idx, &[from]);
        while let Some((k, v)) = iter.next() {
            values.push((k[0], v[0]));
        }
        assert_eq!(values, assumed);
    }

    // Stored
    assert_iter(&fork, 0, &[(10, 10), (20, 20), (30, 30)]);
    assert_iter(&fork, 5, &[(10, 10), (20, 20), (30, 30)]);
    assert_iter(&fork, 10, &[(10, 10), (20, 20), (30, 30)]);
    assert_iter(&fork, 11, &[(20, 20), (30, 30)]);
    assert_iter(&fork, 31, &[]);

    // Inserted
    fork.put(name_idx, vec![5], vec![5]);
    assert_iter(&fork, 0, &[(5, 5), (10, 10), (20, 20), (30, 30)]);
    fork.put(name_idx, vec![25], vec![25]);
    assert_iter(&fork, 0, &[(5, 5), (10, 10), (20, 20), (25, 25), (30, 30)]);
    fork.put(name_idx, vec![35], vec![35]);
    assert_iter(
        &fork,
        0,
        &[(5, 5), (10, 10), (20, 20), (25, 25), (30, 30), (35, 35)],
    );

    // Double inserted
    fork.put(name_idx, vec![25], vec![23]);
    assert_iter(
        &fork,
        0,
        &[(5, 5), (10, 10), (20, 20), (25, 23), (30, 30), (35, 35)],
    );
    fork.put(name_idx, vec![26], vec![26]);
    assert_iter(
        &fork,
        0,
        &[(5, 5), (10, 10), (20, 20), (25, 23), (26, 26), (30, 30), (35, 35)],
    );

    // Replaced
    let mut fork = db.fork();
    let name_idx = fork.get_index(TABLE_NAME);
    fork.put(name_idx, vec![10], vec![11]);
    assert_iter(&fork, 0, &[(10, 11), (20, 20), (30, 30)]);
    fork.put(name_idx, vec![30], vec![31]);
    assert_iter(&fork, 0, &[(10, 11), (20, 20), (30, 31)]);

    // Deleted
    let mut fork = db.fork();
    let name_idx = fork.get_index(TABLE_NAME);
    fork.remove(name_idx, vec![20]);
    assert_iter(&fork, 0, &[(10, 10), (30, 30)]);
    fork.remove(name_idx, vec![10]);
    assert_iter(&fork, 0, &[(30, 30)]);
    fork.put(name_idx, vec![10], vec![11]);
    assert_iter(&fork, 0, &[(10, 11), (30, 30)]);
    fork.remove(name_idx, vec![10]);
    assert_iter(&fork, 0, &[(30, 30)]);

    // MissDeleted
    let mut fork = db.fork();
    let name_idx = fork.get_index(TABLE_NAME);
    fork.remove(name_idx, vec![5]);
    assert_iter(&fork, 0, &[(10, 10), (20, 20), (30, 30)]);
    fork.remove(name_idx, vec![15]);
    assert_iter(&fork, 0, &[(10, 10), (20, 20), (30, 30)]);
    fork.remove(name_idx, vec![35]);
    assert_iter(&fork, 0, &[(10, 10), (20, 20), (30, 30)]);
}

fn changelog<T: Database>(db: T) {
    let mut fork = db.fork();
    let name_idx = fork.get_index(TABLE_NAME);

    fork.put(name_idx, vec![1], vec![1]);
    fork.put(name_idx, vec![2], vec![2]);
    fork.put(name_idx, vec![3], vec![3]);

    assert_eq!(fork.get(name_idx, &[1]), Some(vec![1]));
    assert_eq!(fork.get(name_idx, &[2]), Some(vec![2]));
    assert_eq!(fork.get(name_idx, &[3]), Some(vec![3]));

    fork.checkpoint();

    assert_eq!(fork.get(name_idx, &[1]), Some(vec![1]));
    assert_eq!(fork.get(name_idx, &[2]), Some(vec![2]));
    assert_eq!(fork.get(name_idx, &[3]), Some(vec![3]));

    fork.put(name_idx, vec![1], vec![10]);
    fork.put(name_idx, vec![4], vec![40]);
    fork.remove(name_idx, vec![2]);

    assert_eq!(fork.get(name_idx, &[1]), Some(vec![10]));
    assert_eq!(fork.get(name_idx, &[2]), None);
    assert_eq!(fork.get(name_idx, &[3]), Some(vec![3]));
    assert_eq!(fork.get(name_idx, &[4]), Some(vec![40]));

    fork.rollback();

    assert_eq!(fork.get(name_idx, &[1]), Some(vec![1]));
    assert_eq!(fork.get(name_idx, &[2]), Some(vec![2]));
    assert_eq!(fork.get(name_idx, &[3]), Some(vec![3]));
    assert_eq!(fork.get(name_idx, &[4]), None);

    fork.checkpoint();

    fork.put(name_idx, vec![4], vec![40]);
    fork.put(name_idx, vec![4], vec![41]);
    fork.remove(name_idx, vec![2]);
    fork.put(name_idx, vec![2], vec![20]);

    assert_eq!(fork.get(name_idx, &[1]), Some(vec![1]));
    assert_eq!(fork.get(name_idx, &[2]), Some(vec![20]));
    assert_eq!(fork.get(name_idx, &[3]), Some(vec![3]));
    assert_eq!(fork.get(name_idx, &[4]), Some(vec![41]));

    fork.rollback();

    assert_eq!(fork.get(name_idx, &[1]), Some(vec![1]));
    assert_eq!(fork.get(name_idx, &[2]), Some(vec![2]));
    assert_eq!(fork.get(name_idx, &[3]), Some(vec![3]));
    assert_eq!(fork.get(name_idx, &[4]), None);

    fork.put(name_idx, vec![2], vec![20]);

    fork.checkpoint();

    fork.put(name_idx, vec![3], vec![30]);

    fork.rollback();

    assert_eq!(fork.get(name_idx, &[1]), Some(vec![1]));
    assert_eq!(fork.get(name_idx, &[2]), Some(vec![20]));
    assert_eq!(fork.get(name_idx, &[3]), Some(vec![3]));
    assert_eq!(fork.get(name_idx, &[4]), None);
}


mod memorydb_tests {
    use super::super::MemoryDB;

    fn memorydb_database() -> MemoryDB {
        MemoryDB::new()
    }

    #[test]
    fn test_memory_fork_iter() {
        super::fork_iter(memorydb_database());
    }

    #[test]
    fn test_memory_changelog() {
        super::changelog(memorydb_database());
    }
}

mod rocksdb_tests {
    use std::path::Path;
    use tempdir::TempDir;
    use super::super::{RocksDB, RocksDBOptions};

    fn rocksdb_database(path: &Path) -> RocksDB {
        let mut options = RocksDBOptions::default();
        options.create_if_missing(true);
        RocksDB::open(path, options).unwrap()
    }

    #[test]
    fn test_rocksdb_fork_iter() {
        let dir = TempDir::new("exonum_rocksdb1").unwrap();
        let path = dir.path();
        super::fork_iter(rocksdb_database(path));
    }

    #[test]
    fn test_rocksdb_changelog() {
        let dir = TempDir::new("exonum_rocksdb2").unwrap();
        let path = dir.path();
        super::changelog(rocksdb_database(path));
    }
}
