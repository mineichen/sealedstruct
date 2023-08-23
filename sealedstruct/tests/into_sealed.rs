use sealedstruct_derive::IntoSealed;

#[derive(IntoSealed, PartialEq)]
struct MyGeneric<T: std::fmt::Debug>(T, T);

#[derive(IntoSealed, PartialEq)]
struct MyGeneric2<T>(T, T);
