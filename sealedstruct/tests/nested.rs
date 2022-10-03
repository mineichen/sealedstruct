use std::collections::HashMap;

use config::*;
use sealedstruct::prelude::*;

mod config {
    use std::collections::HashMap;
    use uuid::Uuid;

    // Flaw. Visibility for Raw should be restricted to pub, pub (crate), or pub(super)
    // The hole procedure just makes sense if this is contained in a submodule.
    // All fields of the sealed struct would otherwise be accessible anyway
    #[derive(PartialEq, Default, Debug, sealedstruct::Seal)]
    #[sealedDerive(Clone)]
    pub struct NumbersRaw {
        pub int8: i8,
        pub int16: i16,
        pub int32: i32,
        pub int64: i64,
        pub int128: i128,
    }

    #[derive(PartialEq, Debug, sealedstruct::Seal, sealedstruct::TryIntoSealed)]
    pub(super) struct WrapperRaw {
        pub numbers: NumbersRaw,
        pub ip: std::net::IpAddr,
        pub optional: Option<i32>,
        pub direction: DirectionRaw,
        pub always: AlwaysValid,
        pub map: Vec<NumbersRaw>,
        pub id: Uuid,
        pub hash_map: HashMap<Uuid, NumbersRaw>,
    }

    #[derive(PartialEq, Debug, sealedstruct::Seal, sealedstruct::TryIntoSealed)]
    pub enum DirectionRaw {
        Up,
        Down, //Left(i8),
              //Right { millis: i8 }
    }

    #[derive(PartialEq, Debug, sealedstruct::IntoSealed)]
    pub enum AlwaysValid {
        Bar,
    }

    #[derive(PartialEq, Debug, sealedstruct::IntoSealed)]
    pub struct AlwaysValidStruct {
        foo: i32,
        bar: i32,
    }

    impl Default for WrapperRaw {
        fn default() -> Self {
            Self {
                numbers: Default::default(),
                ip: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                optional: None,
                direction: DirectionRaw::Down,
                always: AlwaysValid::Bar,
                map: Default::default(),
                id: Uuid::from_u128(1),
                hash_map: Default::default(),
            }
        }
    }

    impl sealedstruct::TryIntoSealed for NumbersRaw {
        type Target = NumbersInner;

        fn try_into_sealed(self) -> sealedstruct::Result<Self::Target> {
            NumbersResult {
                int8: if self.int8 < 100 {
                    Ok(self.int8)
                } else {
                    sealedstruct::ValidationError::new("must be <100").into()
                },
                int16: if self.int16 != i16::MAX {
                    Ok(self.int16)
                } else {
                    sealedstruct::ValidationError::new("max is not allowed").into()
                },
                int32: Ok(self.int32),
                int64: Ok(self.int64),
                int128: Ok(self.int128),
            }
            .into()
        }
    }
}

#[derive(PartialEq, Debug, sealedstruct::Seal, sealedstruct::TryIntoSealed)]
pub(crate) struct RootRaw {
    pub child: ChildRaw,
}

#[derive(PartialEq, Debug, sealedstruct::Seal, sealedstruct::TryIntoSealed)]
pub(crate) struct ChildRaw {}

#[test]
fn numbners_is_clone() {
    let value = NumbersRaw::default().seal().unwrap();
    let clone = value.clone();
    assert_eq!(value, clone);
}

#[test]
fn sealed_numbers() {
    let value = NumbersRaw::default().seal().unwrap();
    assert_eq!(NumbersRaw::default(), value);

    let wrapper_sealed = WrapperRaw {
        numbers: NumbersRaw::default(),
        ..Default::default()
    }
    .seal()
    .expect("This should be valid");
    let nr: &NumbersInner = &wrapper_sealed.numbers;

    assert_eq!(0i8, nr.int8);

    assert_ne!(
        NumbersRaw {
            int8: 42,
            ..Default::default()
        },
        NumbersRaw::default().seal().unwrap()
    )
}

#[test]
fn error_path() {
    let r = WrapperRaw {
        numbers: NumbersRaw {
            int8: 127i8,
            int16: i16::MAX,
            ..Default::default()
        },
        ..Default::default()
    }
    .seal();
    match r {
        Ok(_) => panic!("Should be invalid"),
        Err(err) => {
            let mut into_iter = err.into_iter();
            assert_eq!(
                "numbers.int8",
                into_iter
                    .next()
                    .expect("One error")
                    .iter_fields()
                    .next()
                    .expect("Expect one field")
            );
            assert_eq!(
                "numbers.int16",
                into_iter
                    .next()
                    .expect("One error")
                    .iter_fields()
                    .next()
                    .expect("Expect second field")
            );
        }
    }
}

#[test]
fn test_collection_types() {
    #[derive(PartialEq, Debug, sealedstruct::Seal, sealedstruct::TryIntoSealed)]
    pub struct InnerRaw {}
    #[derive(PartialEq, Debug, sealedstruct::Seal, sealedstruct::TryIntoSealed)]
    pub struct OuterRaw {
        map: Option<InnerRaw>,
    }
    OuterResult {
        map: Ok(None), //map: Ok([(1i32, InnerSealed(InnerInner {}))].into_iter().collect()),
    };

    let map = [(1i32, NumbersRaw::default())]
        .into_iter()
        .collect::<HashMap<_, _>>();
    map.seal().unwrap();
}
