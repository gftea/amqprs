//! Utility functions for compliance assert
//! See [Specs](see https://www.rabbitmq.com/resources/specs/amqp0-9-1.extended.xml)

#[inline]
pub(crate) fn assert_regexp(value: &str) {
    // regexp: [a-zA-Z0-9-_.:]
    assert!(value
        .chars()
        .find(|&c| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.' && c != ':')
        .is_none());
}

#[inline]
pub(crate) fn assert_length(value: &str) {
    // max length: 127
    assert!(value.len() < 128);
}

#[inline]
pub(crate) fn assert_notnull(value: &str) {
    assert_ne!(value, "");
}

#[inline]
pub(crate) fn assert_exchange_name(value: &str) {
    assert_length(value);
    assert_regexp(value);
}

#[inline]
pub(crate) fn assert_queue_name(value: &str) {
    assert_length(value);
    assert_regexp(value);
}

#[inline]
pub(crate) fn assert_path(value: &str) {
    assert_notnull(value);
    assert_length(value);
}
