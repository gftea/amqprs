#![allow(unused)]
#![feature(async_iterator)]
fn main() {
    use core::async_iter::AsyncIterator;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    // First, the struct:

    /// An async iterator which counts from one to five
    struct Counter {
        count: usize,
    }

    // we want our count to start at one, so let's add a new() method to help.
    // This isn't strictly necessary, but is convenient. Note that we start
    // `count` at zero, we'll see why in `poll_next()`'s implementation below.
    impl Counter {
        fn new() -> Counter {
            Counter { count: 0 }
        }
    }

    // Then, we implement `AsyncIterator` for our `Counter`:

    impl AsyncIterator for Counter {
        // we will be counting with usize
        type Item = usize;

        // poll_next() is the only required method
        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            // Increment our count. This is why we started at zero.
            self.count += 1;

            // Check to see if we've finished counting or not.
            if self.count < 6 {
                Poll::Ready(Some(self.count))
            } else {
                Poll::Ready(None)
            }
        }
    }

    
}
