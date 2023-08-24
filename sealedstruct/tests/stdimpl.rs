use sealedstruct::Sealable;
use std::collections::{HashMap, HashSet};

#[derive(
    PartialEq, Eq, Hash, Clone, Default, Debug, sealedstruct::Nested, sealedstruct::TryIntoSealed,
)]
pub struct FooRaw {
    x: i32,
}

#[test]
fn compare_hashmap() {
    let map = [(1, FooRaw { x: 42 })]
        .into_iter()
        .collect::<HashMap<i32, FooRaw>>();
    let mut sealed_map: HashMap<i32, Foo> = map.clone().seal().unwrap();

    assert!(map.partial_eq(&sealed_map));
    let mut clone_map = map.clone();
    clone_map.get_mut(&1).unwrap().x = 0;
    assert!(!clone_map.partial_eq(&sealed_map));
    let value = clone_map.remove(&1).unwrap();
    assert!(!clone_map.partial_eq(&sealed_map));
    clone_map.insert(2, value);
    assert!(!clone_map.partial_eq(&sealed_map));

    sealed_map.remove(&1);
    assert!(!map.partial_eq(&sealed_map));
}

#[test]
fn compare_vec() {
    let mut vec = vec![FooRaw { x: 42 }];
    let sealed_vec = vec.clone().seal().unwrap();
    assert!(vec.partial_eq(&sealed_vec));

    vec.push(FooRaw { x: 42 });
    assert!(!vec.partial_eq(&sealed_vec));
    vec.clear();
    assert!(!vec.partial_eq(&sealed_vec));
    vec.push(FooRaw { x: 10 });
}

#[test]
fn compare_hashset() {
    let mut hashset = [42].into_iter().collect::<HashSet<i32>>();
    let sealed_hashset = hashset.clone().seal().unwrap();
    assert!(hashset.partial_eq(&sealed_hashset));

    assert!(hashset.remove(&42));
    assert!(!hashset.partial_eq(&sealed_hashset));
    hashset.insert(2);
    assert!(!hashset.partial_eq(&sealed_hashset));
}

#[test]
fn compare_option() {
    let some = Some(FooRaw { x: 42 });
    let sealed_some = some.clone().seal().unwrap();
    assert!(some.partial_eq(&sealed_some));
    assert!((None as Option<FooRaw>).partial_eq(&None));
    assert!(!some.partial_eq(&None));
    assert!(!(None as Option<FooRaw>).partial_eq(&sealed_some));
    assert!(!Some(FooRaw { x: 0 }).partial_eq(&sealed_some));
}
