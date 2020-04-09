use futures::{
    compat::Stream01CompatExt,
    future::{FutureExt, TryFutureExt},
    stream::TryStreamExt,
};
use futures_old::stream::FuturesUnordered;
use tokio::sync::Semaphore;

#[tokio::main]
async fn main() {
    let sem = Semaphore::new(1);
    let sem = &sem;

    let mut futs = FuturesUnordered::new();
    for _ in 0..200 {
        futs.push(async move {
            let _ = sem.acquire().await;
            Result::<_, ()>::Ok(())
        }.boxed().compat());
    }

    let n = futs.compat().try_collect::<Vec<_>>().await.unwrap().len();
    println!("done: {}", n);
}
