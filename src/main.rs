use futures::{
    compat::Stream01CompatExt,
    future::{FutureExt, TryFutureExt},
    stream::TryStreamExt,
};
use tokio::sync::Semaphore;

mod bounded_traversal {
    use futures_old::{
        stream::{self, FuturesUnordered},
        try_ready, Async, IntoFuture, Stream,
    };
    use std::collections::VecDeque;
    use std::iter::FromIterator;

    pub fn bounded_traversal_stream<In, InsInit, Ins, Out, Unfold, UFut>(
        scheduled_max: usize,
        init: InsInit,
        mut unfold: Unfold,
    ) -> impl Stream<Item = Out, Error = UFut::Error>
    where
        Unfold: FnMut(In) -> UFut,
        UFut: IntoFuture<Item = (Out, Ins)>,
        InsInit: IntoIterator<Item = In>,
        Ins: IntoIterator<Item = In>,
    {
        let mut unscheduled = VecDeque::from_iter(init);
        let mut scheduled = FuturesUnordered::new();
        stream::poll_fn(move || loop {
            if scheduled.is_empty() && unscheduled.is_empty() {
                return Ok(Async::Ready(None));
            }

            for item in unscheduled
                .drain(..std::cmp::min(unscheduled.len(), scheduled_max - scheduled.len()))
            {
                scheduled.push(unfold(item).into_future())
            }

            if let Some((out, children)) = try_ready!(scheduled.poll()) {
                for child in children {
                    unscheduled.push_front(child);
                }
                return Ok(Async::Ready(Some(out)));
            }
        })
    }
}

#[tokio::main]
async fn main() {
    let sem = Semaphore::new(1);
    let sem = &sem;

    let stream = bounded_traversal::bounded_traversal_stream(100, vec![1usize], |i| {
        async move {
            let _ = sem.acquire().await;
            if i > 10 {
                return Result::<_, ()>::Ok(((), vec![]));
            }

            Result::<_, ()>::Ok(((), vec![i + 1, i + 1]))
        }
        .boxed()
        .compat()
    });

    let n = stream.compat().try_collect::<Vec<_>>().await.unwrap().len();
    println!("done: {}", n);
}
