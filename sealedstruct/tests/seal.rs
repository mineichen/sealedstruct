#[test]
fn sealed_numbers_simple() {
    #[derive(PartialEq, Default, Debug, sealedstruct::Seal)]
    pub struct SimpleRaw {
        pub inner: i8,
    }
    impl sealedstruct::Validator for SimpleRaw {
        fn check(&self) -> sealedstruct::Result<()> {
            Ok(())
        }
    }
    let raw = SimpleRaw { inner: 0 };
    let _sealed: Simple = raw.try_into().unwrap();
}

#[test]
fn sealed_numbers_tuple() {
    #[derive(PartialEq, Default, Debug, sealedstruct::Seal)]

    pub struct SimpleTupleRaw(i8);
    impl sealedstruct::Validator for SimpleTupleRaw {
        fn check(&self) -> sealedstruct::Result<()> {
            Ok(())
        }
    }
    sealedstruct::Result::<()>::from(SimpleTupleResult(Result::Ok(()))).unwrap();
    let raw = SimpleTupleRaw(0);
    let _sealed: SimpleTuple = raw.try_into().unwrap();
}

#[test]
fn generic_constrained() {
    #[derive(PartialEq, Default, Debug, sealedstruct::Seal)]
    pub struct SimpleGenericRaw<T: std::fmt::Debug> {
        pub x: i32,
        pub inner: T,
    }

    impl<T: std::fmt::Debug> sealedstruct::Validator for SimpleGenericRaw<T> {
        fn check(&self) -> sealedstruct::Result<()> {
            if self.x == 42 {
                Ok(())
            } else {
                sealedstruct::ValidationError::new("Not 42").into()
            }
        }
    }

    let _: SimpleGeneric<_> = SimpleGenericRaw {
        x: 42,
        inner: "test",
    }
    .seal()
    .unwrap();

    SimpleGenericRaw {
        x: 0,
        inner: "test",
    }
    .seal()
    .unwrap_err();
}
