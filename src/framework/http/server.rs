use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[cfg(feature = "tls")]
use tokio_rustls::TlsAcceptor;

use crate::framework::http::middleware::{MiddlewareChain, MiddlewareFn, HandlerFn};
use crate::framework::http::parser::parse_request;
use crate::framework::http::request::Method;
use crate::framework::http::response::Response;
use crate::framework::http::router::Router;
use crate::framework::error::{FastError, Result};

/// HTTP 服务器
pub struct HttpServer {
    router: Arc<Router>,
    global_middlewares: Vec<MiddlewareFn>,
    #[cfg(feature = "tls")]
    tls_acceptor: Option<TlsAcceptor>,
}

impl HttpServer {
    pub fn new(router: Router) -> Self {
        Self {
            router: Arc::new(router),
            global_middlewares: Vec::new(),
            #[cfg(feature = "tls")]
            tls_acceptor: None,
        }
    }

    /// 添加全局中间件
    pub fn use_middleware(&mut self, mw: MiddlewareFn) {
        self.global_middlewares.push(mw);
    }

    /// 启用 TLS（HTTPS），传入 PEM 格式的证书与私钥文件路径
    #[cfg(feature = "tls")]
    pub fn enable_tls(&mut self, cert_path: &str, key_path: &str) -> Result<()> {
        let cert_pem = std::fs::read(cert_path).map_err(|e| {
            FastError::Http(format!("Failed to read TLS cert '{}': {}", cert_path, e))
        })?;
        let key_pem = std::fs::read(key_path).map_err(|e| {
            FastError::Http(format!("Failed to read TLS key '{}': {}", key_path, e))
        })?;
        self.tls_acceptor = Some(build_tls_acceptor(&cert_pem, &key_pem)?);
        Ok(())
    }

    /// 是否已启用 TLS
    pub fn tls_enabled(&self) -> bool {
        #[cfg(feature = "tls")]
        return self.tls_acceptor.is_some();
        #[cfg(not(feature = "tls"))]
        return false;
    }

    /// 启动服务器（阻塞，直到关闭信号）
    pub async fn serve(self, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| FastError::Http(format!("Failed to bind {}: {}", addr, e)))?;

        let scheme = if self.tls_enabled() { "https" } else { "http" };
        tracing::info!("RustFast HTTP server listening on {}://{}", scheme, addr);

        let router = self.router.clone();
        let middlewares = Arc::new(self.global_middlewares.clone());
        #[cfg(feature = "tls")]
        let tls_acceptor = self.tls_acceptor.clone();

        loop {
            let (stream, peer_addr) = listener.accept().await.map_err(|e| {
                FastError::Http(format!("Accept error: {}", e))
            })?;

            let router = router.clone();
            let middlewares = middlewares.clone();
            #[cfg(feature = "tls")]
            let tls_acceptor = tls_acceptor.clone();

            tokio::spawn(async move {
                #[cfg(feature = "tls")]
                let result = dispatch(stream, peer_addr, router, middlewares, tls_acceptor).await;
                #[cfg(not(feature = "tls"))]
                let result = handle_connection(stream, peer_addr, router, middlewares).await;

                if let Err(e) = result {
                    tracing::debug!("Connection error: {}", e);
                }
            });
        }
    }
}

/// TLS 分流：已启用 TLS 则先完成握手，再按普通连接处理
#[cfg(feature = "tls")]
async fn dispatch(
    stream: TcpStream,
    peer_addr: SocketAddr,
    router: Arc<Router>,
    middlewares: Arc<Vec<MiddlewareFn>>,
    tls_acceptor: Option<TlsAcceptor>,
) -> Result<()> {
    match tls_acceptor {
        Some(acceptor) => match acceptor.accept(stream).await {
            Ok(tls_stream) => handle_connection(tls_stream, peer_addr, router, middlewares).await,
            Err(e) => {
                tracing::debug!("TLS handshake failed from {}: {}", peer_addr, e);
                Ok(())
            }
        },
        None => handle_connection(stream, peer_addr, router, middlewares).await,
    }
}

/// 从 PEM 字节构造 TlsAcceptor
#[cfg(feature = "tls")]
fn build_tls_acceptor(cert_pem: &[u8], key_pem: &[u8]) -> Result<TlsAcceptor> {
    let certs: Vec<_> = rustls_pemfile::certs(&mut &*cert_pem)
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| FastError::Http(format!("Failed to parse TLS cert PEM: {}", e)))?;
    if certs.is_empty() {
        return Err(FastError::Http("TLS cert PEM contains no certificate".into()));
    }

    let key = rustls_pemfile::private_key(&mut &*key_pem)
        .map_err(|e| FastError::Http(format!("Failed to parse TLS key PEM: {}", e)))?
        .ok_or_else(|| FastError::Http("TLS key PEM contains no private key".into()))?;

    let config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| FastError::Http(format!("Invalid TLS cert/key pair: {}", e)))?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// 处理单个连接（TCP 或 TLS 流）
async fn handle_connection<S>(
    mut stream: S,
    peer_addr: SocketAddr,
    router: Arc<Router>,
    middlewares: Arc<Vec<MiddlewareFn>>,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    let remote_addr = peer_addr.to_string();

    let mut request = match parse_request(&mut stream, remote_addr).await {
        Ok(req) => req,
        Err(e) => {
            let resp = Response::fail(400, 400, &format!("Bad Request: {}", e));
            stream.write_all(&resp.to_bytes()).await?;
            return Ok(());
        }
    };

    // OPTIONS 预检请求直接响应
    if request.method == Method::Options {
        let mut resp = Response::new(204);
        resp.set_header("access-control-allow-origin", "*");
        resp.set_header("access-control-allow-methods", "GET,POST,PUT,PATCH,DELETE,OPTIONS");
        resp.set_header("access-control-allow-headers", "content-type,authorization");
        stream.write_all(&resp.to_bytes()).await?;
        return Ok(());
    }

    let response = match router.find(&request.method, &request.path) {
        Some(route_match) => {
            // 填充路径参数
            request.params = route_match.params;

            let handler: HandlerFn = route_match.handler;
            let chain = MiddlewareChain::new(middlewares.as_ref().clone());
            match chain.execute(request, handler).await {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::error!("Handler error: {}", e);
                    match e {
                        FastError::NotFound(msg) => Response::fail(404, 404, &msg),
                        FastError::Unauthorized(msg) => Response::fail(401, 401, &msg),
                        FastError::Validation(msg) => Response::fail(422, 422, &msg),
                        _ => Response::fail(500, 500, "Internal Server Error"),
                    }
                }
            }
        }
        None => {
            if router.has_path(&request.path) {
                Response::fail(405, 405, "Method Not Allowed")
            } else {
                Response::fail(404, 404, "Not Found")
            }
        }
    };

    stream.write_all(&response.to_bytes()).await?;
    stream.flush().await?;
    // 发送 EOF（TLS 流会发送 close_notify，避免对端 read 报 UnexpectedEof）
    let _ = stream.shutdown().await;

    Ok(())
}

#[cfg(all(test, feature = "tls"))]
mod tls_tests {
    use super::*;
    use crate::framework::http::request::Request;
    use tokio::io::AsyncReadExt;
    use tokio_rustls::rustls::client::danger::{
        HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
    };
    use tokio_rustls::rustls::pki_types::{CertificateDer, ServerName, UnixTime};
    use tokio_rustls::rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};

    async fn hello(_req: Request) -> Result<Response> {
        Ok(Response::text(200, "hello tls"))
    }

    /// 测试用：不校验服务端证书
    #[derive(Debug)]
    struct NoVerify;

    impl ServerCertVerifier for NoVerify {
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _server_name: &ServerName<'_>,
            _ocsp_response: &[u8],
            _now: UnixTime,
        ) -> std::result::Result<ServerCertVerified, tokio_rustls::rustls::Error> {
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> std::result::Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> std::result::Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            vec![
                SignatureScheme::ECDSA_NISTP256_SHA256,
                SignatureScheme::ECDSA_NISTP384_SHA384,
                SignatureScheme::ED25519,
                SignatureScheme::RSA_PKCS1_SHA256,
                SignatureScheme::RSA_PKCS1_SHA384,
                SignatureScheme::RSA_PKCS1_SHA512,
                SignatureScheme::RSA_PSS_SHA256,
                SignatureScheme::RSA_PSS_SHA384,
                SignatureScheme::RSA_PSS_SHA512,
            ]
        }
    }

    #[tokio::test]
    async fn test_https_request_response() {
        // 生成自签名证书并写入临时文件
        let rcgen::CertifiedKey { cert, key_pair } =
            rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
        let cert_path = std::env::temp_dir().join(format!("rustfast-test-{}.crt", std::process::id()));
        let key_path = std::env::temp_dir().join(format!("rustfast-test-{}.key", std::process::id()));
        std::fs::write(&cert_path, cert.pem()).unwrap();
        std::fs::write(&key_path, key_pair.serialize_pem()).unwrap();

        // 服务器
        let mut router = Router::new();
        router.get("/hello", hello);
        let mut server = HttpServer::new(router);
        server
            .enable_tls(cert_path.to_str().unwrap(), key_path.to_str().unwrap())
            .unwrap();
        assert!(server.tls_enabled());

        // 选取空闲端口并启动
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let addr = format!("127.0.0.1:{}", port);
        let server_addr = addr.clone();
        tokio::spawn(async move {
            let _ = server.serve(&server_addr).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // 客户端（跳过证书校验）
        let client_config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerify))
            .with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));

        let tcp = TcpStream::connect(&addr).await.unwrap();
        let server_name = ServerName::try_from("localhost").unwrap().to_owned();
        let mut tls = connector.connect(server_name, tcp).await.unwrap();

        tls.write_all(b"GET /hello HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut buf = Vec::new();
        tls.read_to_end(&mut buf).await.unwrap();

        let text = String::from_utf8_lossy(&buf);
        assert!(text.starts_with("HTTP/1.1 200"), "bad status line: {}", text);
        assert!(text.contains("hello tls"), "bad body: {}", text);

        let _ = std::fs::remove_file(&cert_path);
        let _ = std::fs::remove_file(&key_path);
    }
}
