use sealedstruct::IntoNested;

#[derive(IntoNested, PartialEq)]
struct MyGeneric<T: std::fmt::Debug>(T, T);

#[derive(IntoNested, PartialEq)]
struct MyGeneric2<T>(T, T);
