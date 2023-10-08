use std::ops::{Index, IndexMut};

use slotmap::{Key, SecondaryMap, SlotMap};

slotmap::new_key_type! { pub struct NodeIdInternal; }
slotmap::new_key_type! { pub struct InputIdInternal; }
slotmap::new_key_type! { pub struct OutputIdInternal; }

pub type MapId = u128;

#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct UniqueId<K: Key>(K, MapId);

pub type NodeId = UniqueId<NodeIdInternal>;
pub type InputId = UniqueId<InputIdInternal>;
pub type OutputId = UniqueId<OutputIdInternal>;

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AnyParameterId {
    Input(InputId),
    Output(OutputId),
}

impl AnyParameterId {
    pub fn assume_input(&self) -> InputId {
        match self {
            AnyParameterId::Input(input) => *input,
            AnyParameterId::Output(output) => panic!("{:?} is not an InputId", output),
        }
    }
    pub fn assume_output(&self) -> OutputId {
        match self {
            AnyParameterId::Output(output) => *output,
            AnyParameterId::Input(input) => panic!("{:?} is not an OutputId", input),
        }
    }
}

impl From<OutputId> for AnyParameterId {
    fn from(output: OutputId) -> Self {
        Self::Output(output)
    }
}

impl From<InputId> for AnyParameterId {
    fn from(input: InputId) -> Self {
        Self::Input(input)
    }
}

#[derive(Debug, Clone)]
pub struct UniqueSlotmap<K, V>
where
    K: HasKey,
    K::InnerKey: Key,
{
    map: SlotMap<K::InnerKey, V>,
    id: MapId,
}

#[derive(Debug, Clone)]
pub struct UniqueSecondaryMap<K, V>
where
    K: HasKey,
    K::InnerKey: Key,
{
    map: SecondaryMap<K::InnerKey, V>,
    id: MapId,
}

pub trait HasKey {
    type InnerKey;
}

impl<K: Key> HasKey for UniqueId<K> {
    type InnerKey = K;
}

/// Get a random (Semi-unique) ID
fn get_random_map_id() -> MapId {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).unwrap();
    MapId::from_le_bytes(bytes)
}

impl<K, V> Default for UniqueSlotmap<K, V>
where
    K: HasKey,
    K::InnerKey: Key,
{
    fn default() -> Self {
        Self {
            map: SlotMap::with_key(),
            id: get_random_map_id(),
        }
    }
}

impl<K, V> UniqueSecondaryMap<K, V>
where
    K: HasKey,
    K::InnerKey: Key,
{
    pub fn new_from_key<V2>(map: &UniqueSlotmap<K, V2>) -> Self {
        Self {
            map: SecondaryMap::new(),
            id: map.id,
        }
    }
}

impl<K: Key, V> UniqueSlotmap<UniqueId<K>, V> {
    fn check_key(&self, key: UniqueId<K>) -> Option<K> {
        let UniqueId(key, map_id) = key;
        (map_id == self.id).then(|| key)
    }
}

impl<K: Key, V> UniqueSecondaryMap<UniqueId<K>, V> {
    fn check_key(&self, key: UniqueId<K>) -> Option<K> {
        let UniqueId(key, map_id) = key;
        (map_id == self.id).then(|| key)
    }
}

// UniqueSlotmap

impl<K: Key, V> Index<UniqueId<K>> for UniqueSlotmap<UniqueId<K>, V> {
    type Output = V;
    fn index(&self, index: UniqueId<K>) -> &Self::Output {
        if let Some(key) = self.check_key(index) {
            &self.map[key]
        } else {
            panic!("Attempted to access key from another map");
        }
    }
}

impl<K: Key, V> IndexMut<UniqueId<K>> for UniqueSlotmap<UniqueId<K>, V> {
    fn index_mut(&mut self, index: UniqueId<K>) -> &mut Self::Output {
        if let Some(key) = self.check_key(index) {
            &mut self.map[key]
        } else {
            panic!("Attempted to access key from another map");
        }
    }
}

impl<K: Key, V> UniqueSlotmap<UniqueId<K>, V> {
    pub fn get(&self, index: UniqueId<K>) -> Option<&V> {
        self.check_key(index).and_then(|key| self.map.get(key))
    }

    pub fn get_mut(&mut self, index: UniqueId<K>) -> Option<&mut V> {
        let key = self.check_key(index);
        key.and_then(|key| self.map.get_mut(key))
    }

    pub fn remove(&mut self, index: UniqueId<K>) -> Option<V> {
        let key = self.check_key(index);
        key.and_then(|key| self.map.remove(key))
    }

    pub fn iter(&self) -> impl Iterator<Item = (UniqueId<K>, &V)> + '_ {
        self.map.iter().map(move |(k, v)| (UniqueId(k, self.id), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (UniqueId<K>, &mut V)> + '_ {
        let id = self.id;
        self.map.iter_mut().map(move |(k, v)| (UniqueId(k, id), v))
    }

    pub fn insert(&mut self, value: V) -> UniqueId<K> {
        self.insert_with_key(|_| value)
    }

    pub fn insert_with_key<F>(&mut self, f: F) -> UniqueId<K>
    where
        F: FnOnce(UniqueId<K>) -> V,
    {
        let id = self.id;
        let key = self.map.insert_with_key(|callback_key| f(UniqueId(callback_key, id)));
        UniqueId(key, id)
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(UniqueId<K>, &mut V) -> bool,
    {
        let id = self.id;
        self.map.retain(|key, value| f(UniqueId(key, id), value))
    }

    pub fn contains_key(&self, index: UniqueId<K>) -> bool {
        if let Some(key) = self.check_key(index) {
            self.map.contains_key(key)
        } else {
            false
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = UniqueId<K>> + '_ {
        self.map
            .keys()
            .map(move |k| (UniqueId(k, self.id)))
    }
}

// UniqueSecondaryMap

impl<K: Key, V> Index<UniqueId<K>> for UniqueSecondaryMap<UniqueId<K>, V> {
    type Output = V;
    fn index(&self, index: UniqueId<K>) -> &Self::Output {
        if let Some(key) = self.check_key(index) {
            &self.map[key]
        } else {
            panic!("Attempted to access key from another map");
        }
    }
}

impl<K: Key, V> IndexMut<UniqueId<K>> for UniqueSecondaryMap<UniqueId<K>, V> {
    fn index_mut(&mut self, index: UniqueId<K>) -> &mut Self::Output {
        if let Some(key) = self.check_key(index) {
            &mut self.map[key]
        } else {
            panic!("Attempted to access key from another map");
        }
    }
}

impl<K: Key, V> UniqueSecondaryMap<UniqueId<K>, V> {
    pub fn get(&self, index: UniqueId<K>) -> Option<&V> {
        self.check_key(index).and_then(|key| self.map.get(key))
    }

    pub fn get_mut(&mut self, index: UniqueId<K>) -> Option<&mut V> {
        let key = self.check_key(index);
        key.and_then(|key| self.map.get_mut(key))
    }

    pub fn remove(&mut self, index: UniqueId<K>) -> Option<V> {
        let key = self.check_key(index);
        key.and_then(|key| self.map.remove(key))
    }

    pub fn iter(&self) -> impl Iterator<Item = (UniqueId<K>, &V)> + '_ {
        self.map
            .iter()
            .map(move |(k, v)| (UniqueId(k, self.id), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (UniqueId<K>, &mut V)> + '_ {
        let id = self.id;
        self.map.iter_mut().map(move |(k, v)| (UniqueId(k, id), v))
    }

    pub fn insert(&mut self, key: UniqueId<K>, value: V) -> Option<V> {
        if let Some(key) = self.check_key(key) {
            self.map.insert(key, value)
        } else {
            panic!("Attempted to insert key from another map")
        }
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(UniqueId<K>, &mut V) -> bool,
    {
        let id = self.id;
        self.map.retain(|key, value| f(UniqueId(key, id), value))
    }

    pub fn contains_key(&self, index: UniqueId<K>) -> bool {
        if let Some(key) = self.check_key(index) {
            self.map.contains_key(key)
        } else {
            false
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = UniqueId<K>> + '_ {
        self.map
            .keys()
            .map(move |k| (UniqueId(k, self.id)))
    }
}

/*

impl<K, V> Index<K> for UniqueSlotmap<K, V>
where
    K: HasKey,
    K::InnerKey: Key,
{

}

*/
