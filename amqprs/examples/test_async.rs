use async_trait::async_trait;
use tokio::{self, spawn};

#[async_trait]
pub trait Consumer {
    async fn consume(&self);
}

pub struct DefaultConsumer;

#[async_trait]
impl Consumer for DefaultConsumer {
    async fn consume(&self) {
        println!("hello consumer!");
    }
}
struct Test;

impl Test {
    async fn run(mut self) {
        let f = self.level1();
        f.await;
    }
    async fn level1(&mut self) {
        let f = self.level2();
        f.await;
    }

    /// &T - since immutable references can be copied, the ability to send one to another thread
    /// would let you perform immutable access from several threads in parallel.
    /// Thus &T can only be Send if T is Sync. There is no need for T to be Send as an &T doesn't allow mutable access.
    ///
    /// &mut T - mutable references can't be copied, so sending them to other threads doesn't allow
    ///  access from several threads in parallel,
    /// thus &mut T can be Send even if T is not Sync. Of course, T must still be Send.
    async fn level2(&mut self) { // ERROR if change to `&self`
    }
}

#[tokio::main]
async fn main() {
    let t = Test;
    spawn(async move {
        let f = t.run();
        f.await;
    });
}
