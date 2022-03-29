use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    hash::Hash,
};

use smallvec::SmallVec;

pub type Result<T> = std::result::Result<T, ValidationErrors>;
pub use sealedstruct_derive::{IntoSealed, Seal, TryIntoSealed};

pub mod prelude {
    pub use crate::{TryIntoSealed, ValidationResultExtensions};
}

pub trait TryIntoSealed {
    type Target;
    fn try_into_sealed(self) -> Result<Self::Target>;
}

#[derive(Debug, PartialEq, Default, thiserror::Error)]
pub struct ValidationErrors(SmallVec<[ValidationError; 1]>);

// Format is used for summary-purpose only and doesn't output real JSON by choice.
impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("ValidationErrors({}): [", self.0.len()))?;
        let set: HashSet<&str> = self.0.iter().flat_map(|x| x.iter_fields()).collect();
        let mut iter = set.into_iter();
        if let Some(first) = iter.next() {
            f.write_fmt(format_args!("{}{}{}", "", first, ""))?;
            for x in iter.by_ref().take(4) {
                f.write_fmt(format_args!(", {}{}{}", "", x, ""))?;
            }
            if let Some(_) = iter.next() {
                f.write_str(", ...")?;
            }
        }

        f.write_char(']')
    }
}

impl ValidationErrors {
    pub const fn new(error: ValidationError) -> Self {
        Self(SmallVec::from_const([error]))
    }
    pub fn combine_with(mut self, other: ValidationErrors) -> Self {
        self.0.extend(other.0.into_iter());
        self
    }
}

impl Into<ValidationErrors> for ValidationError {
    fn into(self) -> ValidationErrors {
        ValidationErrors::new(self)
    }
}

impl IntoIterator for ValidationErrors {
    type Item = ValidationError;

    type IntoIter = smallvec::IntoIter<[ValidationError; 1]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, PartialEq)]
pub struct ValidationError {
    fields: SmallVec<[String; 1]>,
}

impl ValidationError {
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            fields: SmallVec::from_const([field.into()]),
        }
    }

    pub fn with_fields<T: Into<String>>(first: T, rest: impl IntoIterator<Item = T>) -> Self {
        let mut r = Self::new(first);
        r.fields.extend(rest.into_iter().map(|x| x.into()));
        r
    }

    pub fn iter_fields(&self) -> impl Iterator<Item = &str> {
        self.fields.iter().map(String::as_ref)
    }
}

impl<T> From<ValidationError> for Result<T> {
    fn from(x: ValidationError) -> Self {
        Err(ValidationErrors::new(x))
    }
}

pub trait ValidationResultExtensions {
    type Ok;
    fn combine<T>(self, other: Result<T>) -> Result<(Self::Ok, T)>;
    fn prepend_path(self, path: &str) -> Self;
    fn with_sealed_error(self, error: ValidationError) -> Self;
}

impl<TOwn> ValidationResultExtensions for Result<TOwn> {
    type Ok = TOwn;

    fn combine<T>(self, other: Result<T>) -> Result<(TOwn, T)> {
        match (self, other) {
            (Ok(a), Ok(b)) => Ok((a, b)),
            (Ok(_), Err(e)) => Err(e),
            (Err(e), Ok(_)) => Err(e),
            (Err(a), Err(b)) => Err(a.combine_with(b)),
        }
    }

    fn prepend_path(self, path: &str) -> Self {
        self.map_err(|mut errors| {
            for error in errors.0.iter_mut() {
                for field in error.fields.iter_mut() {
                    field.reserve(path.len() + 1);
                    field.insert(0, '.');
                    field.insert_str(0, path);
                }
            }
            errors
        })
    }

    fn with_sealed_error(self, error: ValidationError) -> Self {
        match self {
            Ok(_) => error.into(),
            Err(prev) => Err(prev.combine_with(ValidationErrors::new(error))),
        }
    }
}

macro_rules! sealed_to_self {
    ($($type:ident),*) => {


        $(
            impl TryIntoSealed for $type {
                type Target = Self;

                fn try_into_sealed(self) -> Result<Self::Target> {
                    Ok(self)
                }
            }
        )*
    };
}
sealed_to_self! {
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    f32, f64,
    usize, isize,
    bool
}

mod std_derives {
    use super::*;
    use std::net::*;
    use std::time::*;

    sealed_to_self! {
        Duration,
        IpAddr, Ipv4Addr, Ipv6Addr
    }
}

impl<TKey, TValue> TryIntoSealed for HashMap<TKey, TValue>
where
    TKey: TryIntoSealed,
    TValue: TryIntoSealed,
    TKey::Target: Hash + Eq,
{
    type Target = HashMap<TKey::Target, TValue::Target>;

    fn try_into_sealed(self) -> Result<Self::Target> {
        self.into_iter()
            .map(|(key, value)| key.try_into_sealed().combine(value.try_into_sealed()))
            .collect()
    }
}

impl<T> TryIntoSealed for Vec<T>
where
    T: TryIntoSealed,
{
    type Target = Vec<T::Target>;

    fn try_into_sealed(self) -> Result<Self::Target> {
        self.into_iter()
            .map(TryIntoSealed::try_into_sealed)
            .collect()
    }
}

impl<T> TryIntoSealed for HashSet<T>
where
    T: TryIntoSealed,
{
    type Target = Vec<T::Target>;

    fn try_into_sealed(self) -> Result<Self::Target> {
        self.into_iter()
            .map(TryIntoSealed::try_into_sealed)
            .collect()
    }
}

#[cfg(feature = "uuid")]
mod uuid_derives {
    use super::*;
    use uuid::Uuid;
    sealed_to_self!(Uuid);
}

impl<T> TryIntoSealed for Option<T>
where
    T: TryIntoSealed,
{
    type Target = Option<T::Target>;

    fn try_into_sealed(self) -> Result<Self::Target> {
        match self {
            Some(x) => x.try_into_sealed().map(Option::Some),
            None => Ok(None),
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::{ValidationError, ValidationErrors};

    use super::prelude::*;

    #[test]
    fn add_error_to_ok_result() {
        let mut result = super::Result::Ok(1);
        result = result.with_sealed_error(ValidationError::new("Foo"));
        let mut errors = result.unwrap_err().into_iter();

        assert_eq!(
            vec!["Foo"],
            errors
                .next()
                .expect("OneError")
                .iter_fields()
                .collect::<Vec<&str>>()
        );
        assert_eq!(errors.next(), None);
    }

    #[test]
    fn add_error_to_err_result() {
        let mut result: super::Result<()> = ValidationError::new("Foo").into();
        result = result.with_sealed_error(ValidationError::new("Bar"));
        let mut errors = result.unwrap_err().into_iter();

        assert_eq!(
            vec!["Foo"],
            errors
                .next()
                .expect("OneError")
                .iter_fields()
                .collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["Bar"],
            errors
                .next()
                .expect("OneError")
                .iter_fields()
                .collect::<Vec<&str>>()
        );
    }

    #[test]
    fn format_validation_error() {
        let result: super::Result<()> = ValidationError::new("Foo").into();
        let result = result.prepend_path("Baz");
        let error: Box<dyn std::error::Error> = Box::new(result.unwrap_err());

        assert_eq!(
            "ValidationErrors(1): [Baz.Foo]".to_string(),
            error.to_string()
        );
    }
}
