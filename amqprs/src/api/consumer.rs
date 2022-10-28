use std::str::from_utf8;

use async_trait::async_trait;

use crate::frame::{BasicPropertities, Deliver};

#[async_trait]
pub trait Consumer {
    async fn consume(
        &mut self,
        deliver: Deliver,
        basic_propertities: BasicPropertities,
        content: Vec<u8>,
    );
}

pub struct DefaultConsumer;

#[async_trait]
impl Consumer for DefaultConsumer {
    async fn consume(
        &mut self,
        deliver: Deliver,
        basic_propertities: BasicPropertities,
        content: Vec<u8>,
    ) {
        println!(">>>>> Consumer Start <<<<<<");
        println!("{:?}", deliver,);
        println!("{:?}", basic_propertities,);
        println!("{}", from_utf8(&content).unwrap());
        println!(">>>>> Consumer End <<<<<<");
    }
}
