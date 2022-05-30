mod stdimpl;
mod wrapper;

use smallvec::SmallVec;
use std::{collections::HashSet, fmt::Write, num, sync::Arc};

pub type Result<T> = std::result::Result<T, ValidationErrors>;
pub use sealedstruct_derive::{IntoSealed, Seal, TryIntoSealed};
pub use wrapper::*;

pub mod prelude {
    pub use crate::{Sealable, ValidationResultExtensions};
}

// Can only be created by the default-Implementation of `Sealable::se`
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Sealed<T>(T);

#[cfg(feature = "serde")]
impl<T: serde::Serialize> serde::Serialize for Sealed<T> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<T> Sealed<T> {
    pub fn new<TRaw: TryIntoSealed<Target = T>>(raw: TRaw) -> Result<Self> {
        Ok(Sealed(raw.try_into_sealed()?))
    }
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Sealed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Usually, converting from Sealed to Raw is straight forward:
/// - When using derive sealedstruct::Seal, Raw implements From<Sealed>, which can be used
/// - When Sealed is == Raw, simply return self
///
/// It gets more complicated for types where Raw cannot implement From<Sealed>
/// - E.g. Generic types like
///
/// Custom types derived from `Seal` usually implement TryIntoSealed only.
/// Sealable is automatically implemented because `Seal`
/// generates PartialEq<Sealed> for Raw and From<Sealed> for Raw
pub trait Sealable {
    type Target;
    fn seal(self) -> Result<Self::Target>;
    fn open(sealed: Self::Target) -> Self;
    // Necessary to compare without cloning
    fn partial_eq(&self, other: &Self::Target) -> bool;
}

pub trait TryIntoSealed {
    type Target;
    fn try_into_sealed(self) -> Result<Self::Target>;
}

impl<T: TryIntoSealed> Sealable for T
where
    T::Target: Into<T>,
    T: PartialEq<T::Target>,
{
    type Target = Sealed<T::Target>;

    fn seal(self) -> Result<Self::Target> {
        Sealed::new(self)
    }

    fn open(sealed: Self::Target) -> Self {
        sealed.0.into()
    }

    fn partial_eq(&self, other: &Self::Target) -> bool {
        self.eq(&other.0)
    }
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
            if iter.next().is_some() {
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

impl From<ValidationError> for ValidationErrors {
    fn from(e: ValidationError) -> Self {
        ValidationErrors::new(e)
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
    ($($type:ty),*) => {
        $(
            impl Sealable for $type {
                type Target = Self;

                fn seal(self) -> Result<Self> {
                    Ok(self)
                }

                fn open(sealed: Self) -> Self {
                    sealed
                }

                fn partial_eq(&self, other: &Self) -> bool {
                    self.eq(other)
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
    num::NonZeroU8, num::NonZeroU16, num::NonZeroU32, num::NonZeroU64, num::NonZeroU128,
    num::NonZeroI8, num::NonZeroI16, num::NonZeroI32, num::NonZeroI64, num::NonZeroI128,
    bool,
    &'static str,
    String
}

impl<T0: Sealable, T1: Sealable> Sealable for (T0, T1) {
    type Target = (T0::Target, T1::Target);

    fn seal(self) -> Result<Self::Target> {
        Ok((self.0.seal()?, self.1.seal()?))
    }

    fn open(sealed: Self::Target) -> Self {
        (T0::open(sealed.0), T1::open(sealed.1))
    }

    fn partial_eq(&self, other: &Self::Target) -> bool {
        self.0.partial_eq(&other.0) && self.1.partial_eq(&other.1)
    }
}
impl<T0: Sealable, T1: Sealable, T2: Sealable> Sealable for (T0, T1, T2) {
    type Target = (T0::Target, T1::Target, T2::Target);

    fn seal(self) -> Result<Self::Target> {
        Ok((self.0.seal()?, self.1.seal()?, self.2.seal()?))
    }

    fn open(sealed: Self::Target) -> Self {
        (T0::open(sealed.0), T1::open(sealed.1), T2::open(sealed.2))
    }

    fn partial_eq(&self, other: &Self::Target) -> bool {
        self.0.partial_eq(&other.0) && self.1.partial_eq(&other.1) && self.2.partial_eq(&other.2)
    }
}

impl<T: Sealable + Clone> Sealable for Arc<T>
where
    T::Target: Clone,
{
    type Target = Arc<T::Target>;

    fn seal(self) -> Result<Self::Target> {
        T::clone(&self).seal().map(Arc::new)
    }

    fn open(sealed: Self::Target) -> Self {
        Arc::new(Sealable::open(T::Target::clone(&sealed)))
    }

    fn partial_eq(&self, other: &Self::Target) -> bool {
        <T as Sealable>::partial_eq(self, other)
    }
}

mod std_derives {
    use super::*;

    sealed_to_self! {
        std::time::Duration,
        std::net::IpAddr,
        std::net::Ipv4Addr,
        std::net::Ipv6Addr
    }
}
#[cfg(feature = "uuid")]
mod uuid_derives {
    use super::*;
    sealed_to_self! {
        uuid::Uuid
    }
}

#[cfg(feature = "chrono")]
mod uuid_derives {
    use super::*;
    sealed_to_self! {
        chrono::DateTime<chrono::Utc>
    }
}

#[cfg(test)]
mod tests {
    use crate::ValidationError;

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
