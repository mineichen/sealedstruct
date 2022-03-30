use crate::{Result, TryIntoSealedExtended, ValidationResultExtensions};
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    hash::Hash,
};

impl<TKey, TValue> TryIntoSealedExtended for HashMap<TKey, TValue>
where
    TKey: TryIntoSealedExtended + Hash + Eq,
    TValue: TryIntoSealedExtended,
    TKey::Target: Hash + Eq + Borrow<TKey>,
{
    type Target = HashMap<TKey::Target, TValue::Target>;

    fn try_into_sealed_extended(self) -> Result<Self::Target> {
        self.into_iter()
            .map(|(key, value)| {
                key.try_into_sealed_extended()
                    .combine(value.try_into_sealed_extended())
            })
            .collect()
    }

    fn from_sealed(sealed: Self::Target) -> Self {
        sealed
            .into_iter()
            .map(|(key, value)| (TKey::from_sealed(key), TValue::from_sealed(value)))
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
        return true;
    }
}

impl<T> TryIntoSealedExtended for Vec<T>
where
    T: TryIntoSealedExtended,
{
    type Target = Vec<T::Target>;

    fn try_into_sealed_extended(self) -> Result<Self::Target> {
        self.into_iter()
            .map(TryIntoSealedExtended::try_into_sealed_extended)
            .collect()
    }

    fn from_sealed(sealed: Self::Target) -> Self {
        sealed
            .into_iter()
            .map(|value| T::from_sealed(value))
            .collect()
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

impl<T> TryIntoSealedExtended for HashSet<T>
where
    T: TryIntoSealedExtended + Hash + Eq,
    T::Target: Hash + Eq + Borrow<T>,
{
    type Target = HashSet<T::Target>;

    fn try_into_sealed_extended(self) -> Result<Self::Target> {
        self.into_iter()
            .map(TryIntoSealedExtended::try_into_sealed_extended)
            .collect()
    }
    fn from_sealed(sealed: Self::Target) -> Self {
        sealed
            .into_iter()
            .map(|value| T::from_sealed(value))
            .collect()
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
        return true;
    }
}

impl<T> TryIntoSealedExtended for Option<T>
where
    T: TryIntoSealedExtended,
{
    type Target = Option<T::Target>;

    fn try_into_sealed_extended(self) -> Result<Self::Target> {
        match self {
            Some(x) => x.try_into_sealed_extended().map(Option::Some),
            None => Ok(None),
        }
    }
    fn from_sealed(sealed: Self::Target) -> Self {
        sealed.map(|value| T::from_sealed(value))
    }

    fn partial_eq(&self, other: &Self::Target) -> bool {
        match (self, other) {
            (None, None) => true,
            (None, Some(_)) | (Some(_), None) => false,
            (Some(a), Some(b)) => a.partial_eq(&b),
        }
    }
}
