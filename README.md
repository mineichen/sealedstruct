# Validation

This crate generates boilerplate code to get from an bare (unchecked/mutable) to a sealed (checked/immutable) state.
The sealed state can always be transformed back to it's bare state. Transition from bare to sealed could fail with ValidationErrors.
With this construct, creating invalid sealed state is avoided at compiletime.

```
struct RelativeRange {
    from: f32, 
    to: f32
}

struct RawRelativeRange {

}

```