// FAIL fixture for RESIL-FAILURE-MODE-TEST.1: a trust-boundary handler is
// defined but no companion test asserts its error path is handled.

pub async fn handle_request(req: Request) -> Response {
    let body = req.body();
    process(body)
}

fn process(body: Vec<u8>) -> Response {
    Response::ok(body)
}
