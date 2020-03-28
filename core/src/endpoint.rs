use crate::packet::ResponseWindow;
use crate::pki::Certificate;
use crate::pki::CertificateId;
use crate::Connection;
use crate::ConnectionError;
use crate::Database;
use crate::EndpointError;
use futures::channel::mpsc::UnboundedSender;
use futures::prelude::*;
use http::StatusCode;
use quinn::CertificateChain;
use quinn::ClientConfig;
use quinn::Endpoint;
use quinn::NewConnection;
use quinn::ServerConfig;
use rustls::internal::msgs::handshake::DistinguishedNames;
use rustls::ClientCertVerified;
use rustls::ClientCertVerifier;
use rustls::RootCertStore;
use rustls::ServerCertVerified;
use rustls::ServerCertVerifier;
use rustls::TLSError;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use webpki::DNSName;
use webpki::DNSNameRef;

pub struct Config<'a> {
    pub certificate: &'a [u8],
    pub key: &'a [u8],
    pub database: &'a Arc<dyn Database>,
}

/// QUIC endpoint that binds to all local interfaces in the network.
///
/// This also serves as the main endpoint that is used to connect with remote [Node](crate::Node)s.
pub struct LocalEndpoint {
    quic: Endpoint,
    client_config: ClientConfig,
}

impl LocalEndpoint {
    pub fn start(
        config: &Config,
    ) -> Result<(Self, impl Stream<Item = NewConnection>), EndpointError> {
        let cert_chain = CertificateChain::from_certs(std::iter::once(
            quinn::Certificate::from_der(&config.certificate)?,
        ));
        let quinn_key = quinn::PrivateKey::from_der(config.key)?;
        let verifier = Arc::new(CertificateVerifier {
            account_id: config.certificate.id(),
            database: config.database.clone(),
        });

        let mut server_config = ServerConfig::default();
        server_config.certificate(cert_chain.clone(), quinn_key)?;
        let server_tls_config = Arc::get_mut(&mut server_config.crypto).unwrap();
        server_tls_config.set_client_certificate_verifier(verifier.clone());

        let rustls_key = rustls::PrivateKey(config.key.into());
        let mut client_config = ClientConfig::default();
        let client_tls_config = Arc::get_mut(&mut client_config.crypto).unwrap();
        client_tls_config.set_single_client_cert(cert_chain.into_iter().collect(), rustls_key)?;
        client_tls_config
            .dangerous()
            .set_certificate_verifier(verifier);

        let mut endpoint_builder = Endpoint::builder();
        endpoint_builder.listen(server_config);
        let socket = UdpSocket::bind("[::]:0")?;
        let (endpoint, incoming) = endpoint_builder.with_socket(socket)?;
        log::info!(
            "Started local endpoint on port {}",
            endpoint.local_addr().unwrap().port()
        );

        let stream = incoming
            .then(|connecting| connecting)
            .inspect(|connecting| {
                if let Err(err) = connecting {
                    log::error!("Failed to process an incoming connection: {}", err)
                }
            })
            .filter_map(|connecting| async { connecting.ok() });

        Ok((
            Self {
                quic: endpoint,
                client_config,
            },
            stream,
        ))
    }

    pub async fn connect(&self, addr: &SocketAddr) -> Result<NewConnection, ConnectionError> {
        self.quic
            .connect_with(self.client_config.clone(), addr, "Viska Node")?
            .await
            .map_err(Into::into)
    }
}

pub trait ConnectionInfo {
    /// Gets the account ID of the [Node](crate::Node) who opened this connection.
    ///
    /// Consult [AuthenticationData](quinn::crypto::rustls::AuthenticationData) for the option-ness.
    fn account_id(&self) -> Option<CertificateId>;
    fn remote_address(&self) -> SocketAddr;
}

impl ConnectionInfo for quinn::Connection {
    fn remote_address(&self) -> SocketAddr {
        self.remote_address()
    }

    fn account_id(&self) -> Option<CertificateId> {
        self.authentication_data()
            .peer_certificates
            .and_then(|chain| chain.iter().next().map(|cert| cert.id()))
    }
}

impl From<quinn::Connection> for Connection {
    fn from(src: quinn::Connection) -> Self {
        Self {
            quic: src,
            id: Uuid::new_v4(),
        }
    }
}

impl ConnectionInfo for Connection {
    fn remote_address(&self) -> SocketAddr {
        self.quic.remote_address()
    }

    fn account_id(&self) -> Option<CertificateId> {
        self.quic.account_id()
    }
}

pub struct ConnectionManager {
    connections: RwLock<HashMap<Uuid, Arc<Connection>>>,
    endpoint: LocalEndpoint,
    response_window_sink: UnboundedSender<ResponseWindow>,
}

impl ConnectionManager {
    pub fn new(
        endpoint: LocalEndpoint,
        response_window_sink: UnboundedSender<ResponseWindow>,
    ) -> Self {
        Self {
            endpoint,
            response_window_sink,
            connections: Default::default(),
        }
    }

    pub async fn add(self: Arc<Self>, new_quic_connection: NewConnection) -> Arc<Connection> {
        let connection = Arc::<Connection>::new(new_quic_connection.connection.into());
        self.connections
            .write()
            .await
            .insert(connection.id, connection.clone());

        let connection_cloned = connection.clone();
        let task = new_quic_connection
            .bi_streams
            .inspect(|bi_stream| {
                if let Err(err) = bi_stream {
                    log::error!("Failed to accept an incoming QUIC stream: {}", err);
                }
            })
            .filter_map(|bi_stream| async { bi_stream.ok() })
            .for_each(move |(sender, receiver)| {
                let connection_manager = self.clone();
                let connection = connection_cloned.clone();
                let mut response_window_sink = self.response_window_sink.clone();
                async move {
                    let window = ResponseWindow::new(
                        connection_manager.clone(),
                        connection.clone(),
                        sender,
                        receiver,
                    )
                    .await;
                    if let Some(w) = window {
                        response_window_sink.send(w).await.unwrap_or_else(|err| {
                            log::error!("Failed to create a ResponseWindow: {}", err)
                        });
                    }
                }
            });
        tokio::spawn(task);
        log::info!("Connected by {}", connection.remote_address());
        connection
    }

    pub async fn close(&self, id: &Uuid, code: StatusCode) {
        let connection = self
            .connections
            .write()
            .await
            .remove(id)
            .unwrap_or_else(|| panic!("Double closing connection {}", &id));
        log::info!("Closing connection to {}", connection.remote_address());
        connection.quic.close(code.as_u16().into(), &[]);
    }

    pub async fn connect(
        self: Arc<Self>,
        addr: &SocketAddr,
    ) -> Result<Arc<Connection>, ConnectionError> {
        Ok(self.clone().add(self.endpoint.connect(addr).await?).await)
    }
}

/// Certification verifier for Viska's protocols.
///
/// Only 2 kinds of [Node](cate::Node)s are allowed to connect with us:
///
/// * Device: Those with the same certificate as us.
/// * Peer: Those whose certificate ID is in our roster.
struct CertificateVerifier {
    account_id: CertificateId,
    database: Arc<dyn Database>,
}

impl CertificateVerifier {
    fn verify(&self, presented_certs: &[rustls::Certificate]) -> Result<(), TLSError> {
        // TODO: Check expiration
        match presented_certs {
            [cert] => {
                let peer_id = cert.id();
                if self.account_id == peer_id || self.database.is_peer(self.account_id.as_bytes()) {
                    Ok(())
                } else {
                    Err(TLSError::General("Unrecognized certificate ID".into()))
                }
            }
            [] => Err(TLSError::NoCertificatesPresented),
            _ => Err(TLSError::PeerMisbehavedError(
                "Only 1 certificate is allowed in the chain".into(),
            )),
        }
    }
}

impl ClientCertVerifier for CertificateVerifier {
    fn client_auth_root_subjects(&self, _: Option<&DNSName>) -> Option<DistinguishedNames> {
        Some(Default::default())
    }
    fn verify_client_cert(
        &self,
        presented_certs: &[rustls::Certificate],
        _: Option<&DNSName>,
    ) -> Result<ClientCertVerified, TLSError> {
        self.verify(presented_certs)
            .map(|_| ClientCertVerified::assertion())
    }
}

impl ServerCertVerifier for CertificateVerifier {
    fn verify_server_cert(
        &self,
        _: &RootCertStore,
        presented_certs: &[rustls::Certificate],
        _: DNSNameRef,
        _: &[u8],
    ) -> Result<ServerCertVerified, TLSError> {
        self.verify(presented_certs)
            .map(|_| ServerCertVerified::assertion())
    }
}