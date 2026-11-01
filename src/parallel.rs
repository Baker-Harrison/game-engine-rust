use std::thread;

pub fn thread_count() -> usize {
    thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

pub fn for_each_chunk<T: Send>(items: &mut [T], granularity: usize, work: impl Fn(usize, &mut [T]) + Sync) {
    if items.is_empty() {
        return;
    }
    let granules = items.len().div_ceil(granularity);
    let chunk_len = granules.div_ceil(thread_count()) * granularity;
    thread::scope(|scope| {
        for (index, chunk) in items.chunks_mut(chunk_len).enumerate() {
            let work = &work;
            scope.spawn(move || work(index * chunk_len, chunk));
        }
    });
}
