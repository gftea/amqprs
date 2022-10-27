use async_trait::async_trait;

use crate::frame::{BasicPropertities, Deliver};

#[async_trait]
pub trait Consumer {
    async fn consume(
        &self,
        deliver: Deliver,
        basic_propertities: BasicPropertities,
        content: Vec<u8>,
    );
}

pub struct DefaultConsumer;

#[async_trait]
impl Consumer for DefaultConsumer {
    async fn consume(
        &self,
        deliver: Deliver,
        basic_propertities: BasicPropertities,
        content: Vec<u8>,
    ) {
        println!("{:?}, {:?}, {:?}", deliver, basic_propertities, content);
    }
}