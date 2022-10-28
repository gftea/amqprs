use std::{future::Future, pin::Pin};
use tokio::time;
use tokio_stream::{Stream, StreamExt};

struct MyOwnFuture {
    inner: time::Interval,
    now: time::Instant,
}

impl MyOwnFuture {
    fn new() -> Self {
        Self {
            inner: time::interval(time::Duration::from_millis(500)),
            now: time::Instant::now(),
        }
    }
}
impl Stream for MyOwnFuture {
    type Item = time::Instant;

    // NOTE: we have change `self` to `mut self`, this is OK because we take the ownership!
    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.inner.poll_tick(cx) {
            std::task::Poll::Ready(value) => match value.checked_duration_since(self.now) {
                Some(d) if d.as_secs() < 5 => std::task::Poll::Ready(Some(value.clone())),
                Some(_d) => std::task::Poll::Ready(None),

                None => std::task::Poll::Pending,
            },
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
impl Future for MyOwnFuture {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        // `async` return a future that implements !Unpin
        unsafe {
            Pin::new_unchecked(&mut async {
                let inner = &mut self.get_mut().inner;
                loop {
                    let i = inner.tick().await;

                    println!("tick: {:?}", i);
                }
            })
        }
        .poll(cx)
    }
}

#[tokio::main]
async fn main() {
    let mut fut = MyOwnFuture::new();
    // fut.await;
    while let Some(v) = fut.next().await {
        println!("tick at {:?}", v)
    }
    println!("stream exhausted")
}
