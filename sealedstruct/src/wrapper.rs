use std::ops::{Deref, DerefMut};

use crate::Sealable;

/// Used to wrap Values you have no control over
/// It delegates most standard traits to it's inner component
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Copy, Default)]
pub struct IntoNestedWrapper<T>(T);

impl<T: PartialEq> Sealable for IntoNestedWrapper<T> {
    type Target = IntoNestedWrapper<T>;

    fn seal(self) -> crate::Result<Self::Target> {
        Ok(self)
    }

    fn open(sealed: Self::Target) -> Self {
        sealed
    }

    fn partial_eq(&self, other: &Self::Target) -> bool {
        self.0 == other.0
    }
}

impl<T: PartialEq> Deref for IntoNestedWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: PartialEq> DerefMut for IntoNestedWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: PartialEq> From<T> for IntoNestedWrapper<T> {
    fn from(i: T) -> Self {
        IntoNestedWrapper(i)
    }
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> serde::Serialize for IntoNestedWrapper<T> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}
#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for IntoNestedWrapper<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(IntoNestedWrapper)
    }
}
