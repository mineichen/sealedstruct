# Validation

This crate generates boilerplate code to get from an bare (unchecked/mutable) to a sealed (checked/immutable) state.
The sealed state can always be transformed back to it's bare state. Transition from bare to sealed could fail with ValidationErrors.
With this construct, creating invalid sealed state is avoided at compiletime.

There are two macros available for different purpose. They might be merged in the future


The API is very experimental and can currently break at any time. This is why there is no version on crates.io yet.


## With intermediate Representation
```rust
use sealedstruct::{Nested, TryIntoNested, ValidationError};

#[derive(sealedstruct::Nested)]
pub struct FooNestedRaw {
    x: i32
}

impl TryIntoNested for FooNestedRaw {
    type Target = FooNestedInner;
    fn try_into_nested(self) -> sealedstruct::Result<Self::Target> {
        FooNestedResult {
            x: if self.x <= 42 { Ok(self.x) } else { ValidationError::new("Value must be smaller than ").into()}
        }.into()
    }
}



#[derive(sealedstruct::Seal, Debug)]
pub struct RelativeRangeRaw {
    from: Percentage,
    to: Percentage
}
impl sealedstruct::Validator for RelativeRangeRaw {
    fn check(&self) -> sealedstruct::Result<()> {
        if self.from < self.to {
            Ok(())
        } else {
           ValidationError::on_fields("from", ["to"], "From must be smaller than to").into()
        }

    }
}

#[derive(sealedstruct::Seal, Debug, PartialEq, PartialOrd )]
pub struct PercentageRaw(f32);


impl sealedstruct::Validator for PercentageRaw {
    fn check(&self) -> sealedstruct::Result<()> {
        PercentageResult(if matches!(self.0, 0.0..=1.0) { 
                Ok(()) 
            } else { 
                ValidationError::new(format!("Percentages must be between 0 and 1, got {}", self.0)).into()
            }            
        ).into()
    }
}

let mut errors = RelativeRangeRaw {
    from: PercentageRaw(0.9).seal().unwrap(),
    to: PercentageRaw(0.1).seal().unwrap(),    
}.seal().unwrap_err().into_iter();
let Some(e) = errors.next() else {
    panic!("Should contain at least one error");
};
assert_eq!("From must be smaller than to", e.reason);
assert_eq!(None, errors.next());

```

If all errors should be available,