use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_callback() {
    let (tx, mut rx) = mpsc::channel(1);

    let callback = |s: &'static str| async move {
        println!("async callback closure: {}", s);
    };
    match tx.send(callback).await {
        Ok(_) => (),
        Err(_) => todo!(),
    }
    match rx.recv().await {
        Some(cb) => cb("test").await,
        None => todo!(),
    }

    let callback2 = |s: &'static str| async move {
        println!("async callback: {}", s);
        println!("async callback: {}", s);
    };
    // match  tx.send(callback2).await {
    //     Ok(_) => (),
    //     Err(_) => todo!(),
    // }
    // match rx.recv().await {
    //     Some(cb) => cb("test").await,
    //     None => todo!(),
    // }
}

fn send_callback<F>(callback: F)
where
    F: FnMut() -> (),
{
}
