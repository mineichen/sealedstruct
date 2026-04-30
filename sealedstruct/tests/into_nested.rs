use sealedstruct::IntoNested;

#[allow(dead_code)]
#[derive(IntoNested, PartialEq)]
struct MyGeneric<T: std::fmt::Debug>(T, T);

#[allow(dead_code)]
#[derive(IntoNested, PartialEq)]
struct MyGeneric2<T>(T, T);
