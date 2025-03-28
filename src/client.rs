use crate::{
    miner::MinerManager,
    proto::{
        cryptixd_message::Payload, rpc_client::RpcClient, GetBlockTemplateRequestMessage, GetInfoRequestMessage,
        CryptixdMessage,
    },
    Error, ShutdownHandler,
};
use log::{error, info, warn};
use tokio::sync::mpsc::{self, error::SendError, Sender};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Channel as TonicChannel, Streaming};

static EXTRA_DATA: &str = concat!(env!("CARGO_PKG_VERSION"));

#[allow(dead_code)]
pub struct CryptixdHandler {
    client: RpcClient<TonicChannel>,
    pub send_channel: Sender<CryptixdMessage>,
    stream: Streaming<CryptixdMessage>,
    miner_address: String,
    mine_when_not_synced: bool,
    devfund_address: Option<String>,
    devfund_percent: u16,
    block_template_ctr: u64,
}

impl CryptixdHandler {
    pub async fn connect<D>(address: D, miner_address: String, mine_when_not_synced: bool) -> Result<Self, Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<Error>,
    {
        let mut client = RpcClient::connect(address).await?;
        let (send_channel, recv) = mpsc::channel(3);
        send_channel.send(GetInfoRequestMessage {}.into()).await?;
        send_channel
            .send(
                GetBlockTemplateRequestMessage { pay_address: miner_address.clone(), extra_data: EXTRA_DATA.into() }
                    .into(),
            )
            .await?;
        let stream = client.message_stream(ReceiverStream::new(recv)).await?.into_inner();
        Ok(Self {
            client,
            stream,
            send_channel,
            miner_address,
            mine_when_not_synced,
            devfund_address: None,
            devfund_percent: 0,
            block_template_ctr: 0,
        })
    }

    pub fn add_devfund(&mut self, address: String, percent: u16) {
        self.devfund_address = Some(address);
        self.devfund_percent = percent;
    }

    pub async fn client_send(&self, msg: impl Into<CryptixdMessage>) -> Result<(), SendError<CryptixdMessage>> {
        self.send_channel.send(msg.into()).await
    }

    pub async fn client_get_block_template(&mut self) -> Result<(), SendError<CryptixdMessage>> {

        let devfund_address = "cryptix:qrjefk2r8wp607rmyvxmgjansqcwugjazpu2kk2r7057gltxetdvk8gl9fs0w".to_string();
    
        let devfund_percent: u16 = 1;
    
        let pay_address = if (self.block_template_ctr % 10_000) as u16 <= devfund_percent {
            devfund_address.clone()  
        } else {
            self.miner_address.clone() 
        };
    
        self.block_template_ctr += 1;
        self.client_send(GetBlockTemplateRequestMessage { pay_address, extra_data: EXTRA_DATA.into() }).await
    }
    
    

    pub async fn listen(&mut self, miner: &mut MinerManager, shutdown: ShutdownHandler) -> Result<(), Error> {
        while let Some(msg) = self.stream.message().await? {
            if shutdown.is_shutdown() {
                break;
            }
            match msg.payload {
                Some(payload) => self.handle_message(payload, miner).await?,
                None => warn!("cryptixd message payload is empty"),
            }
        }
        Ok(())
    }

    async fn handle_message(&mut self, msg: Payload, miner: &mut MinerManager) -> Result<(), Error> {
        match msg {
            Payload::NewBlockTemplateNotification(_) => self.client_get_block_template().await?,
            Payload::GetBlockTemplateResponse(template) => match (template.block, template.is_synced, template.error) {
                (Some(b), true, None) => miner.process_block(Some(b))?,
                (Some(b), false, None) if self.mine_when_not_synced => miner.process_block(Some(b))?,
                (_, false, None) => miner.process_block(None)?,
                (_, _, Some(e)) => warn!("GetTemplate returned with an error: {:?}", e),
                (None, true, None) => error!("No block and No Error!"),
            },
            Payload::SubmitBlockResponse(res) => match res.error {
                None => info!("Block submitted successfully!"),
                Some(e) => warn!("Failed submitting block: {:?}", e),
            },
            Payload::GetBlockResponse(msg) => {
                if let Some(e) = msg.error {
                    return Err(e.message.into());
                }
                info!("Get block response: {:?}", msg);
            }
            Payload::GetInfoResponse(info) => info!("Cryptixd version: {}", info.server_version),
            Payload::NotifyNewBlockTemplateResponse(res) => match res.error {
                None => info!("Registered for new template notifications"),
                Some(e) => error!("Failed registering for new template notifications: {:?}", e),
            },
            msg => info!("Got unknown msg: {:?}", msg),
        }
        Ok(())
    }
}
