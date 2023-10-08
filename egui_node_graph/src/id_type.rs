use slotmap::{Key, SlotMap, SecondaryMap};

slotmap::new_key_type! { pub struct NodeIdInternal; }
slotmap::new_key_type! { pub struct InputIdInternal; }
slotmap::new_key_type! { pub struct OutputIdInternal; }

pub type MapId = u64;

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
}

trait HasKey {
    type InnerKey;
}

impl<K: Key> HasKey for UniqueId<K> {
    type InnerKey = K;
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

fn get_random_map_id() -> MapId {
    let mut bytes = [0u8; 8];
    getrandom::getrandom(&mut bytes);
    u64::from_le_bytes(bytes)
}
