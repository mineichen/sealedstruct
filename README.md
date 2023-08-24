# Validation

This crate generates boilerplate code to get from an bare (unchecked/mutable) to a sealed (checked/immutable) state.
The sealed state can always be transformed back to it's bare state. Transition from bare to sealed could fail with ValidationErrors.
With this construct, creating invalid sealed state is avoided at compiletime.

There are two macros available for different purpose. They might be merged in the future


The API is very experimental and can currently break at any time. This is why there is no version on crates.io yet.


## With intermediate Representation
```rust
#[derive(sealedstruct::Nested)]
struct FooRaw {
    x: i32
}

impl TryIntoNested for ShapeDetectionRaw {
    type Target = FooInner;
    fn try_into_nested(self) -> sealedstruct::Result<Self::Target> {
        todo!("Your validation logic goes here")    
    }
}

#[derive(sealedstruct::Seal)]
struct FooSimpleRaw {
    x: i32
}
impl sealedstruct::Validator for InvertibleTransform3dRaw {
    fn check(&self) -> sealedstruct::Result<()> {
        todo!("Your validation logic goes here");
    }
}
```
