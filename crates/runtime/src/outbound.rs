use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutboundOutcome {
    pub status: u16,
    pub transferred_bytes: u64,
}

pub type OutboundFuture<'a> =
    Pin<Box<dyn Future<Output = Result<OutboundOutcome, ()>> + Send + 'a>>;

pub trait OutboundRuntime: Send + Sync {
    fn get_status<'a>(&'a self, target: &'a str, path: &'a str) -> OutboundFuture<'a>;
    fn post_json_status<'a>(
        &'a self,
        target: &'a str,
        path: &'a str,
        body: &'a [u8],
    ) -> OutboundFuture<'a>;
}
