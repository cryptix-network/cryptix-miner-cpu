use crate::{
    pow::{self, HeaderHasher},
    proto::{
        cryptixd_message::Payload, GetBlockTemplateRequestMessage, GetInfoRequestMessage, CryptixdMessage,
        NotifyBlockAddedRequestMessage, NotifyNewBlockTemplateRequestMessage, RpcBlock, SubmitBlockRequestMessage,
    },
    Hash,
};

impl CryptixdMessage {
    #[must_use]
    #[inline(always)]
    pub fn get_info_request() -> Self {
        CryptixdMessage { payload: Some(Payload::GetInfoRequest(GetInfoRequestMessage {})) }
    }
    #[must_use]
    #[inline(always)]
    pub fn notify_block_added() -> Self {
        CryptixdMessage { payload: Some(Payload::NotifyBlockAddedRequest(NotifyBlockAddedRequestMessage {})) }
    }
    #[must_use]
    #[inline(always)]
    pub fn submit_block(block: RpcBlock) -> Self {
        CryptixdMessage {
            payload: Some(Payload::SubmitBlockRequest(SubmitBlockRequestMessage {
                block: Some(block),
                allow_non_daa_blocks: false,
            })),
        }
    }
}

impl From<GetInfoRequestMessage> for CryptixdMessage {
    #[inline(always)]
    fn from(a: GetInfoRequestMessage) -> Self {
        CryptixdMessage { payload: Some(Payload::GetInfoRequest(a)) }
    }
}
impl From<NotifyBlockAddedRequestMessage> for CryptixdMessage {
    #[inline(always)]
    fn from(a: NotifyBlockAddedRequestMessage) -> Self {
        CryptixdMessage { payload: Some(Payload::NotifyBlockAddedRequest(a)) }
    }
}

impl From<GetBlockTemplateRequestMessage> for CryptixdMessage {
    #[inline(always)]
    fn from(a: GetBlockTemplateRequestMessage) -> Self {
        CryptixdMessage { payload: Some(Payload::GetBlockTemplateRequest(a)) }
    }
}

impl From<NotifyNewBlockTemplateRequestMessage> for CryptixdMessage {
    fn from(a: NotifyNewBlockTemplateRequestMessage) -> Self {
        CryptixdMessage { payload: Some(Payload::NotifyNewBlockTemplateRequest(a)) }
    }
}

impl RpcBlock {
    #[must_use]
    #[inline(always)]
    pub fn block_hash(&self) -> Option<Hash> {
        let mut hasher = HeaderHasher::new();
        pow::serialize_header(&mut hasher, self.header.as_ref()?, false);
        Some(hasher.finalize())
    }
}
