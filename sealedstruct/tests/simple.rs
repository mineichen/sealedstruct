#[derive(PartialEq, Default, Debug, sealedstruct::SealSimple, sealedstruct::Validator)]
pub struct SimpleRaw {
    pub inner: i8,
}

#[test]
fn sealed_numbers_simple() {
    let raw = SimpleRaw { inner: 0 };
    let _sealed: Simple = raw.try_into().unwrap();
}

#[test]
fn sealed_numbers_tuple() {
    #[derive(PartialEq, Default, Debug, sealedstruct::SealSimple, sealedstruct::Validator)]
    pub struct SimpleTupleRaw(i8);
    sealedstruct::Result::<()>::from(SimpleTupleResult(Result::Ok(()))).unwrap();
    let raw = SimpleTupleRaw(0);
    let _sealed: SimpleTuple = raw.try_into().unwrap();
}
