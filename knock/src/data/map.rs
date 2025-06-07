use serde::de::{DeserializeOwned, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::hash::Hash;
use std::marker::PhantomData;

/// An abstraction over a hashmap, allowing the key to be a part of the item itself.
///
/// The data is serialized and deserialized as a list.
#[derive(Debug, Clone)]
pub struct Map<T: MapItem>(HashMap<T::Key, T>);

pub trait MapItem: Serialize + DeserializeOwned {
    type Key: Clone + Eq + Hash;

    fn key(&self) -> &Self::Key;
}

impl<T: MapItem> Map<T> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&T>
    where
        T::Key: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.0.get(key)
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut T>
    where
        T::Key: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.0.get_mut(key)
    }

    pub fn get_or_insert_with<Q>(&mut self, key: &Q, insert: impl FnOnce() -> T) -> &mut T
    where
        T::Key: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if !self.0.contains_key(key) {
            let item = insert();
            self.insert(item);
        }
        self.0.get_mut(key).unwrap()
    }

    pub fn insert(&mut self, item: T) {
        self.0.insert(item.key().clone(), item);
    }
}

impl<T: MapItem> Serialize for Map<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;

        for item in self.0.values() {
            seq.serialize_element(&item)?;
        }

        seq.end()
    }
}

impl<'de, T: MapItem> Deserialize<'de> for Map<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visit<T>(PhantomData<T>);

        impl<'de, T: MapItem> Visitor<'de> for Visit<T> {
            type Value = Map<T>;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                write!(formatter, "a list")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut map = Map::new();
                while let Some(item) = seq.next_element()? {
                    map.insert(item);
                }

                Ok(map)
            }
        }

        deserializer.deserialize_seq(Visit(PhantomData))
    }
}

impl<T: MapItem> Default for Map<T> {
    fn default() -> Self {
        Self::new()
    }
}
