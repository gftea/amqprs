use std::collections::BTreeMap;

type MyMap = BTreeMap<String, Box<dyn Fn() -> ()>>;

fn generic_func<F>(f: F)
where
    F: Fn() -> () + 'static,
{
    let mut map: MyMap = BTreeMap::new();
    let v = Box::new(f);
    map.insert("key".to_string(), v);

    map.remove("key").unwrap()();
}

fn concrete_func(f: Box<dyn Fn() -> ()>) {
    let mut map: MyMap = BTreeMap::new();
    map.insert("key".to_string(), f);
    map.remove("key").unwrap()();

}
fn main() {
    generic_func(|| {
        println!("generic func call");
    });

    concrete_func(Box::new(|| {
        println!("concrete func call");
    }));
}
