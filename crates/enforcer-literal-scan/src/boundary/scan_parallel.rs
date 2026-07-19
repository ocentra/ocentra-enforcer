use std::thread;

use crate::discovery::{chunk_jobs, scan_file};
use crate::scan_types::{FileJob, FileResult};

pub(crate) fn scan_jobs_in_parallel(jobs: Vec<FileJob>) -> Vec<FileResult> {
    let thread_count = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(4)
        .max(1);
    let chunks = chunk_jobs(jobs, thread_count);
    let mut results = Vec::new();

    thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in chunks {
            handles.push(scope.spawn(move || scan_chunk(chunk)));
        }
        for handle in handles {
            if let Ok(mut local) = handle.join() {
                results.append(&mut local);
            }
        }
    });

    results
}

fn scan_chunk(chunk: Vec<FileJob>) -> Vec<FileResult> {
    let mut local = Vec::new();
    for job in chunk {
        if let Ok(result) = scan_file(job) {
            local.push(result);
        }
    }
    local
}
