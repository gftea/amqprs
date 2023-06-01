use amqprs::FieldName;
use amqprs::FieldTable;
use amqprs::FieldValue;

fn main() {
    let mut ft = FieldTable::new();

    ft.insert("x-message-ttl".try_into().unwrap(), FieldValue::l(60000));
}
