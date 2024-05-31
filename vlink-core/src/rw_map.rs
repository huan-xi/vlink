use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};
use std::sync::{Arc};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

// pub enum Entry<'a, K, V, S = RandomState> {
//     Occupied(OccupiedEntry<'a, K, V, S>),
//     Vacant(VacantEntry<'a, K, V, S>),
// }

// #[derive(Clone)]
pub struct RwMap<K, V> {
    pub inter: Arc<RwLock<HashMap<K, V>>>,
}

impl<K, V> Clone for RwMap<K, V> {
    fn clone(&self) -> Self {
        RwMap {
            inter: self.inter.clone(),
        }
    }
}

impl<K: Eq + Hash, V> From<Vec<(K, V)>> for RwMap<K, V> {
    fn from(value: Vec<(K, V)>) -> Self {
        RwMap {
            inter: Arc::new(RwLock::new(value.into_iter().collect())),
        }
    }
}

impl<K, V> From<HashMap<K, V>> for RwMap<K, V> {
    fn from(map: HashMap<K, V>) -> Self {
        RwMap {
            inter: Arc::new(RwLock::new(map)),
        }
    }
}

pub struct RefEntry<'a, K, V> {
    lock: RwLockWriteGuard<'a, HashMap<K, V>>,
    // entry: Entry<'a, K, V>,
}

impl<K, V> Default for RwMap<K, V>
    where
        K: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> RwMap<K, V>
    where K: Eq + Hash {
    pub fn new() -> Self {
        RwMap {
            inter: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn insert(&self, key: K, value: V) -> Option<V> {
        self.inter.write().await.insert(key, value)
    }

    pub async fn remove(&self, key: &K) -> Option<V> {
        self.inter.write().await.remove(key)
    }
    pub async fn read_lock(&self) -> RwLockReadGuard<'_, HashMap<K, V>> {
        self.inter.read().await
    }
    pub async fn write_lock(&self) -> RwLockWriteGuard<'_, HashMap<K, V>> {
        self.inter.write().await
    }
    // pub async fn entry(&self, key: K) -> Entry<'_, K, V> {
    //     let mut write = self.inter.write().await;
    //     write.entry(key)
    // }
    /* pub async fn entry<'a>(&'a self, key: K) -> RefEntry<'a, K, V> {
         let mut write = self.inter.write().await;
         let entry = write.entry(key);

         let entry = match entry {
             Entry::Occupied(e) => Entry::Occupied(e),
             Entry::Vacant(v) => Entry::Vacant(v),
         };
         RefEntry {
             lock: write,
             // entry,
         }
     }*/
}

