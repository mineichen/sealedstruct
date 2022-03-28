use config::*;
use sealedstruct::prelude::*;

mod config {
    use sealedstruct::TryIntoSealed;

    // Flaw. Visibility for Raw should be restricted to pub, pub (crate), or pub(super)
    // The hole procedure just makes sense if this is contained in a submodule.
    // All fields of the sealed struct would otherwise be accessible anyway
    #[derive(PartialEq, Default, Debug, sealedstruct::Seal)]
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
    }

    #[derive(PartialEq, Debug, sealedstruct::Seal, sealedstruct::TryIntoSealed)]
    pub enum DirectionRaw {
        Up,
        Down, //Left(i8),
              //Right { millis: i8 }
    }

    #[derive(PartialEq, Debug, sealedstruct::IntoSealed)]
    pub enum AlwaysValid {
        Foo,
        Bar,
    }

    impl Default for WrapperRaw {
        fn default() -> Self {
            Self {
                numbers: Default::default(),
                ip: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                optional: None,
                direction: DirectionRaw::Down,
                always: AlwaysValid::Bar,
            }
        }
    }

    impl sealedstruct::TryIntoSealed for NumbersRaw {
        type Target = NumbersSealed;

        fn try_into_sealed(self) -> sealedstruct::Result<Self::Target> {
            NumbersResult {
                int8: if self.int8 < 100 {
                    Ok(self.int8)
                } else {
                    sealedstruct::ValidationError::new("int8").into()
                },
                int16: Ok(self.int16),
                int32: Ok(self.int32),
                int64: Ok(self.int64),
                int128: Ok(self.int128),
            }
            .into()
        }
    }
}
#[test]
fn sealed_numbers() {
    let value = NumbersRaw::default().try_into_sealed().unwrap();
    assert_eq!(NumbersRaw::default(), value);

    let wrapper_sealed = WrapperRaw {
        numbers: NumbersRaw::default(),
        ..Default::default()
    }
    .try_into_sealed()
    .expect("This should be valid");
    let nr: &NumbersSealed = &wrapper_sealed.numbers;
    assert_eq!(0i8, nr.int8);

    assert_ne!(
        NumbersRaw {
            int8: 42,
            ..Default::default()
        },
        NumbersRaw::default().try_into_sealed().unwrap()
    )
}

#[test]
fn error_path() {
    let r = WrapperRaw {
        numbers: NumbersRaw {
            int8: 127i8,
            ..Default::default()
        },
        ..Default::default()
    }
    .try_into_sealed();
    match r {
        Ok(_) => panic!("Should be invalid"),
        Err(err) => {
            let mut into_iter = err.into_iter();
            let error = into_iter.next().expect("One error");
            assert_eq!(
                "numbers.int8",
                error.iter_fields().next().expect("Expect one field")
            );
        }
    }
}
