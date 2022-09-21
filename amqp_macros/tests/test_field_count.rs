use amqp_macros::FieldCount;

#[test]
fn it_works() {
    #[allow(dead_code)]
    #[derive(FieldCount)]
    struct Test {
        a: i32,
        b: String,
        c: Vec<u8>,
    }
    assert_eq!(3, Test::field_count());
}



