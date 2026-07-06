// PASS fixture for RESIL-FAILURE-MODE-TEST.1: the trust-boundary handler
// has a companion failure-mode test asserting its failure path is handled.

pub async fn handle_request(req: Request) -> Response {
    let body = req.body();
    process(body)
}

fn process(body: Vec<u8>) -> Response {
    Response::ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_invalid_body() {
        let req = Request::with_body(vec![]);
        let response = futures::executor::block_on(handle_request(req));
        assert_err!(response.status());
    }
}
