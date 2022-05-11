use crate::{Result, Sealable, ValidationResultExtensions};
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    hash::Hash,
};

impl<TKey, TValue> Sealable for HashMap<TKey, TValue>
where
    TKey: Sealable + Hash + Eq,
    TValue: Sealable,
    TKey::Target: Hash + Eq + Borrow<TKey>,
{
    type Target = HashMap<TKey::Target, TValue::Target>;

    fn seal(self) -> Result<Self::Target> {
        self.into_iter()
            .map(|(key, value)| key.seal().combine(value.seal()))
            .collect()
    }

    fn open(sealed: Self::Target) -> Self {
        sealed
            .into_iter()
            .map(|(key, value)| (TKey::open(key), TValue::open(value)))
            .collect()
    }

    fn partial_eq(&self, other: &Self::Target) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for (key, value) in self.iter() {
            if let Some(other_value) = other.get(key) {
                if !value.partial_eq(other_value) {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
}

impl<T> Sealable for Vec<T>
where
    T: Sealable,
{
    type Target = Vec<T::Target>;

    fn seal(self) -> Result<Self::Target> {
        self.into_iter().map(Sealable::seal).collect()
    }

    fn open(sealed: Self::Target) -> Self {
        sealed.into_iter().map(|value| T::open(value)).collect()
    }

    fn partial_eq(&self, other: &Self::Target) -> bool {
        let samelen = self.len() == other.len();
        let mut self_iter = self.iter();
        let mut other_iter = other.iter();

        samelen
            && self_iter
                .by_ref()
                .zip(other_iter.by_ref())
                .fold(true, |acc, (a, b)| acc && a.partial_eq(b))
    }
}

impl<T> Sealable for HashSet<T>
where
    T: Sealable + Hash + Eq,
    T::Target: Hash + Eq + Borrow<T>,
{
    type Target = HashSet<T::Target>;

    fn seal(self) -> Result<Self::Target> {
        self.into_iter().map(Sealable::seal).collect()
    }
    fn open(sealed: Self::Target) -> Self {
        sealed.into_iter().map(|value| T::open(value)).collect()
    }

    fn partial_eq(&self, other: &Self::Target) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for value in self.iter() {
            if other.get(value) == None {
                return false;
            }
        }
        true
    }
}

impl<T> Sealable for Option<T>
where
    T: Sealable,
{
    type Target = Option<T::Target>;

    fn seal(self) -> Result<Self::Target> {
        match self {
            Some(x) => x.seal().map(Option::Some),
            None => Ok(None),
        }
    }
    fn open(sealed: Self::Target) -> Self {
        sealed.map(|value| T::open(value))
    }

    fn partial_eq(&self, other: &Self::Target) -> bool {
        match (self, other) {
            (None, None) => true,
            (None, Some(_)) | (Some(_), None) => false,
            (Some(a), Some(b)) => a.partial_eq(b),
        }
    }
}
