use quinn::{RecvStream, SendStream};
use wincode::error::{ReadResult, WriteResult};
use wincode::{SchemaRead, SchemaWrite};

type Username = String;
type Password = String;

#[derive(SchemaRead, SchemaWrite)]
#[derive(Debug)]
pub enum ClientMessage {
    Ping,
    Work,
    GetBalance,
    GetFlag
}

#[derive(SchemaRead, SchemaWrite)]
pub enum ServerResponse {
    Pong,
    Worked(u32),
    Balance(u32),
    BadWork(String),
    YouAreTooPoor,
    Flag(String)
}

impl ClientMessage {
    pub fn serialize(&self) -> WriteResult<Vec<u8>> {
        wincode::serialize(self)
    }

    pub fn deserialize(data: &[u8]) -> ReadResult<Self> {
        wincode::deserialize(data)
    }
}

impl ServerResponse {
    pub fn serialize(&self) -> WriteResult<Vec<u8>> {
        wincode::serialize(self)
    }

    pub fn deserialize(data: &[u8]) -> ReadResult<Self> {
        wincode::deserialize(data)
    }
}

pub async fn send_frame(tx: &mut SendStream, data: &[u8]) -> anyhow::Result<()> {
    let size = data.len();
    let size_buf = size.to_le_bytes();
    tx.write(&size_buf).await?;
    tx.write(data).await?;
    Ok(())
}

pub async fn recv_frame(rx: &mut RecvStream) -> anyhow::Result<Vec<u8>> {
    let mut size_buf = [0u8; 4];
    rx.read_exact(&mut size_buf).await?;
    let size = u32::from_le_bytes(size_buf);
    let mut data_buf = vec![0u8; size as usize];
    rx.read_exact(&mut data_buf).await?;
    Ok(data_buf.to_vec())
}