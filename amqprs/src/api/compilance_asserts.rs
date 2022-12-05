//! Utility functions for compilance assert
//! See [Specs](see https://www.rabbitmq.com/resources/specs/amqp0-9-1.extended.xml)


pub fn assert_amqp_exchange_name(value: &str) {
    // max length: 127
    assert!(value.len() < 128);
    // regexp: [a-zA-Z0-9-_.:]
    assert!(value
        .chars()
        .find(|&c| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.' && c != ':')
        .is_none());
}

pub fn assert_amqp_path(value: &str) {
    // not null
    assert_ne!(value, "");
    // max length: 127
    assert!(value.len() < 128);

}