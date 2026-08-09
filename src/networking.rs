use crate::cli::Args;
use crate::constants::{FLAG_COST, MAX_BI_STREAMS, MAX_IDLE_TIMEOUT, MONEY_PER_WORK};
use crate::messages::{ClientMessage, ServerResponse, recv_frame, send_frame};
use crate::util::HumanDuration;
use anyhow::anyhow;
use quinn::rustls::pki_types::PrivateKeyDer;
use quinn::{Connection, Endpoint, RecvStream, SendStream, ServerConfig, TransportConfig, VarInt};
use rcgen::generate_simple_self_signed;
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{Instrument, Span, error, info, instrument, warn};

pub struct Server {
    endpoint: Endpoint,
    ip_limit_enabled: bool,
}

impl Server {
    pub async fn new(args: &Args) -> anyhow::Result<Self> {
        let conf = make_conf()?;
        let bind_addr = SocketAddr::new(args.host, args.port);
        let endpoint = Endpoint::server(conf, bind_addr)?;

        info!("Server initialized");
        Ok(Self {
            endpoint,
            ip_limit_enabled: !args.disable_ip_limit,
        })
    }

    #[instrument(skip(self))]
    pub async fn run(self) {
        let addr_str = match self.endpoint.local_addr() {
            Ok(addr) => addr.to_string(),
            Err(_) => "(unknown)".to_string(),
        };
        info!("Server listening on {addr_str}");

        let connected_ips: Arc<RwLock<HashSet<IpAddr>>> = Default::default();

        while let Some(incoming) = self.endpoint.accept().await {
            info!("Incoming connection: {:?}", incoming.remote_address());
            if self.ip_limit_enabled
                && connected_ips
                    .read()
                    .await
                    .contains(&incoming.remote_address().ip())
            {
                warn!(
                    "Refusing connection from {} as this address already has an open connection",
                    incoming.remote_address()
                );
                incoming.refuse();
                continue;
            }

            match incoming.await {
                Ok(conn) => {
                    info!("Accepted connection from {}", conn.remote_address());
                    let conn_ip = conn.remote_address().ip();
                    let ips = connected_ips.clone();
                    tokio::spawn(async move {
                        ips.write().await.insert(conn_ip);

                        if let Err(e) = Self::handle_connection(conn).await {
                            error!(%e, "Error occured during connection handling");
                        }

                        ips.write().await.remove(&conn_ip);
                    });
                }
                Err(e) => {
                    error!(%e, "Connection failed");
                }
            }
        }
    }

    #[instrument(skip_all, fields(addr=conn.remote_address().to_string()))]
    async fn handle_connection(conn: Connection) -> anyhow::Result<()> {
        info!("Started connection handler");

        let money = Arc::new(AtomicU32::new(0));
        while let Ok((tx, rx)) = conn.accept_bi().await {
            info!("Incoming stream");
            let money_cloned = money.clone();
            tokio::spawn(
                async move {
                    if let Err(e) = Self::handle_stream(tx, rx, money_cloned).await {
                        error!(%e, "Error occurred during stream handling");
                    }
                }
                .instrument(Span::current()),
            );
        }
        info!("Connection handler closed");
        Ok(())
    }

    #[instrument(skip_all)]
    async fn handle_stream(
        mut tx: SendStream,
        mut rx: RecvStream,
        money: Arc<AtomicU32>,
    ) -> anyhow::Result<()> {
        info!("Handling stream");

        let mut can_work_at = Instant::now();
        while let Ok(m) = recv_msg(&mut rx).await {
            info!("Incoming message: {:?}", m);

            let now = Instant::now();
            let resp = match m {
                ClientMessage::Ping => ServerResponse::Pong,
                ClientMessage::Work => {
                    if now < can_work_at {
                        let diff = can_work_at - now;
                        ServerResponse::BadWork(format!(
                            "You can work again in {}",
                            diff.human_duration()
                        ))
                    } else {
                        can_work_at = Instant::now() + Duration::from_hours(3);
                        money.fetch_add(MONEY_PER_WORK, Ordering::Relaxed);
                        ServerResponse::Worked(MONEY_PER_WORK)
                    }
                }
                ClientMessage::GetBalance => ServerResponse::Balance(money.load(Ordering::Acquire)),
                ClientMessage::GetFlag => {
                    if money.load(Ordering::Acquire) < FLAG_COST {
                        ServerResponse::YouAreTooPoor
                    } else {
                        ServerResponse::Flag(
                            std::env::var("FLAG").unwrap_or("FLAG NOT LOADED".to_string()),
                        )
                    }
                }
            };

            info!("Responding with {:?}", resp);
            if let Err(e) = send_msg(&mut tx, resp).await {
                warn!(%e, "Failed to respond to stream");
            }
        }
        info!("Done handling stream");
        Ok(())
    }
}

async fn recv_msg(rx: &mut RecvStream) -> anyhow::Result<ClientMessage> {
    ClientMessage::deserialize(recv_frame(rx).await?.as_slice())
        .map_err(|e| anyhow!("Failed to receive message: {e}"))
}

async fn send_msg(tx: &mut SendStream, msg: ServerResponse) -> anyhow::Result<()> {
    send_frame(tx, msg.serialize()?.as_slice())
        .await
        .map_err(|e| anyhow!("Failed to send message: {e}"))
}

fn make_conf() -> anyhow::Result<ServerConfig> {
    let cert = generate_simple_self_signed(vec!["localhost".to_string()])?;
    let cert_der = cert.cert.der().clone();
    let key_der = cert.signing_key.serialize_der();

    let mut conf =
        ServerConfig::with_single_cert(vec![cert_der], PrivateKeyDer::Pkcs8(key_der.into()))?;
    configure_transport(&mut conf);

    Ok(conf)
}

fn configure_transport(conf: &mut ServerConfig) {
    let mut transport = TransportConfig::default();
    transport.max_concurrent_bidi_streams(MAX_BI_STREAMS.into());
    transport.max_concurrent_uni_streams(0u32.into());
    transport.max_idle_timeout(Some(
        VarInt::from_u32(MAX_IDLE_TIMEOUT.as_millis() as u32).into(),
    ));
    conf.transport_config(Arc::new(transport));
}
