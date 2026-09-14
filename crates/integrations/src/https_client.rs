use crate::egress::{EgressPolicy, Target, ip_allowed};
use crate::egress_capability::{EgressCapability, EgressEndpoint, HttpsPath};
use crate::error::IntegrationError;
use crate::http_response::{HttpsResponse, StatusResponse, read_http_response};
use crate::secrets::SecretString;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, lookup_host};
use tokio_rustls::TlsConnector;

const MAX_VELRAN_STATUS_BODY_BYTES: usize = 256 * 1024;

#[derive(Clone)]
pub struct OutboundHttpsClient {
    policy: EgressPolicy,
    tls: TlsConnector,
}

impl OutboundHttpsClient {
    pub fn new(policy: EgressPolicy) -> Self {
        let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let cfg = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        Self {
            policy,
            tls: TlsConnector::from(Arc::new(cfg)),
        }
    }

    pub fn capability(&self, name: &str) -> Result<EgressCapability, IntegrationError> {
        self.policy.capability(name)
    }

    pub fn validate_velran_target(&self, target_name: &str) -> Result<(), IntegrationError> {
        self.sole_endpoint(target_name).map(|_| ())
    }

    pub async fn get_status_with_usage(
        &self,
        target_name: &str,
        path_and_query: &str,
    ) -> Result<StatusResponse, IntegrationError> {
        let endpoint = self.sole_endpoint(target_name)?;
        let path = HttpsPath::new(path_and_query.to_string())?;
        self.request_status(&endpoint, "GET", &path, &[], &[]).await
    }

    pub async fn post_json_status_with_usage(
        &self,
        target_name: &str,
        path_and_query: &str,
        body: &[u8],
    ) -> Result<StatusResponse, IntegrationError> {
        let endpoint = self.sole_endpoint(target_name)?;
        let path = HttpsPath::new(path_and_query.to_string())?;
        self.request_status(
            &endpoint,
            "POST",
            &path,
            body,
            &[("Content-Type", "application/json")],
        )
        .await
    }

    async fn request_status(
        &self,
        endpoint: &EgressEndpoint,
        method: &str,
        path: &HttpsPath,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<StatusResponse, IntegrationError> {
        let target = endpoint.target.clone();
        if body.len() > target.max_sent_bytes {
            return Err(IntegrationError::SendTooLarge);
        }
        for (key, value) in headers {
            validate_header(key, value)?;
        }
        let response = tokio::time::timeout(
            target.total_timeout,
            self.request_inner_with_body_cap(
                &target,
                &endpoint.host,
                endpoint.port,
                method,
                path.as_str(),
                body,
                headers,
                target.max_received_bytes.min(MAX_VELRAN_STATUS_BODY_BYTES),
            ),
        )
        .await
        .map_err(|_| IntegrationError::Timeout)??;
        Ok(StatusResponse {
            status: response.status,
            transferred_bytes: response.body.len() as u64,
        })
    }

    fn sole_endpoint(&self, target_name: &str) -> Result<EgressEndpoint, IntegrationError> {
        let target = self.policy.target(target_name)?;
        if target.hosts.len() != 1 || target.ports.len() != 1 {
            return Err(IntegrationError::Policy(
                "Velran outbound call target must resolve to exactly one configured host and port"
                    .into(),
            ));
        }
        self.capability(target_name)?
            .endpoint(&target.hosts[0], target.ports[0])
    }

    pub async fn get(
        &self,
        endpoint: &EgressEndpoint,
        path_and_query: &HttpsPath,
    ) -> Result<HttpsResponse, IntegrationError> {
        self.request(endpoint, "GET", path_and_query, &[], &[])
            .await
    }

    pub async fn post_json(
        &self,
        endpoint: &EgressEndpoint,
        path_and_query: &HttpsPath,
        body: &[u8],
        bearer: Option<&SecretString>,
    ) -> Result<HttpsResponse, IntegrationError> {
        let mut headers = vec![("Content-Type", "application/json")];
        let bearer_header;
        if let Some(secret) = bearer {
            let token = std::str::from_utf8(secret.bytes())
                .map_err(|_| IntegrationError::Secret("bearer secret is not UTF-8".into()))?;
            if token.bytes().any(|b| matches!(b, b'\r' | b'\n' | 0)) {
                return Err(IntegrationError::Secret("invalid bearer secret".into()));
            }
            bearer_header = format!("Bearer {token}");
            headers.push(("Authorization", bearer_header.as_str()));
        }
        self.request(endpoint, "POST", path_and_query, body, &headers)
            .await
    }

    async fn request(
        &self,
        endpoint: &EgressEndpoint,
        method: &str,
        path: &HttpsPath,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<HttpsResponse, IntegrationError> {
        let target = endpoint.target.clone();
        if body.len() > target.max_sent_bytes {
            return Err(IntegrationError::SendTooLarge);
        }
        for (key, value) in headers {
            validate_header(key, value)?;
        }
        tokio::time::timeout(
            target.total_timeout,
            self.request_inner(
                &target,
                &endpoint.host,
                endpoint.port,
                method,
                path.as_str(),
                body,
                headers,
            ),
        )
        .await
        .map_err(|_| IntegrationError::Timeout)?
    }

    async fn request_inner(
        &self,
        target: &Target,
        host: &str,
        port: u16,
        method: &str,
        path: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<HttpsResponse, IntegrationError> {
        self.request_inner_with_body_cap(
            target,
            host,
            port,
            method,
            path,
            body,
            headers,
            target.max_received_bytes,
        )
        .await
    }

    async fn request_inner_with_body_cap(
        &self,
        target: &Target,
        host: &str,
        port: u16,
        method: &str,
        path: &str,
        body: &[u8],
        headers: &[(&str, &str)],
        response_body_cap: usize,
    ) -> Result<HttpsResponse, IntegrationError> {
        let answers: Vec<SocketAddr> = lookup_host((host, port))
            .await
            .map_err(|_| IntegrationError::Dns)?
            .collect();
        if answers.is_empty() || answers.len() > target.max_dns_answers {
            return Err(IntegrationError::Policy("DNS answer count denied".into()));
        }
        let mut approved = Vec::new();
        for addr in answers {
            if !ip_allowed(&target.cidrs, addr.ip()) {
                return Err(IntegrationError::Policy(
                    "DNS answer outside target CIDR".into(),
                ));
            }
            if !approved.contains(&addr) {
                approved.push(addr);
            }
        }
        let mut last_connect_err = None;
        for addr in approved {
            match tokio::time::timeout(target.connect_timeout, TcpStream::connect(addr)).await {
                Ok(Ok(stream)) => {
                    let peer = stream.peer_addr().map_err(|_| IntegrationError::Connect)?;
                    if peer.ip() != addr.ip() || !ip_allowed(&target.cidrs, peer.ip()) {
                        return Err(IntegrationError::Policy("connected peer IP denied".into()));
                    }
                    return self
                        .tls_http(
                            target,
                            host,
                            port,
                            stream,
                            method,
                            path,
                            body,
                            headers,
                            response_body_cap,
                        )
                        .await;
                }
                _ => last_connect_err = Some(IntegrationError::Connect),
            }
        }
        Err(last_connect_err.unwrap_or(IntegrationError::Connect))
    }

    async fn tls_http(
        &self,
        target: &Target,
        host: &str,
        port: u16,
        stream: TcpStream,
        method: &str,
        path: &str,
        body: &[u8],
        headers: &[(&str, &str)],
        response_body_cap: usize,
    ) -> Result<HttpsResponse, IntegrationError> {
        let server_name =
            ServerName::try_from(host.to_owned()).map_err(|_| IntegrationError::Tls)?;
        let mut stream = self
            .tls
            .connect(server_name, stream)
            .await
            .map_err(|_| IntegrationError::Tls)?;
        let mut request = Vec::new();
        request.extend_from_slice(
            format!(
                "{method} {path} HTTP/1.1\r\nHost: {}\r\nUser-Agent: velran-m14/0.1\r\nAccept: application/json\r\nConnection: close\r\n",
                host_header(host, port)
            )
            .as_bytes(),
        );
        for (key, value) in headers {
            request.extend_from_slice(format!("{key}: {value}\r\n").as_bytes());
        }
        if !body.is_empty() || method == "POST" {
            request.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
        }
        request.extend_from_slice(b"\r\n");
        request.extend_from_slice(body);
        if request.len() > target.max_sent_bytes {
            return Err(IntegrationError::SendTooLarge);
        }
        stream
            .write_all(&request)
            .await
            .map_err(|_| IntegrationError::Connect)?;
        stream
            .flush()
            .await
            .map_err(|_| IntegrationError::Connect)?;
        read_http_response(&mut stream, response_body_cap).await
    }
}

fn host_header(host: &str, port: u16) -> String {
    if port == 443 {
        host.to_string()
    } else {
        format!("{host}:{port}")
    }
}

fn validate_header(k: &str, v: &str) -> Result<(), IntegrationError> {
    if k.is_empty()
        || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || v.bytes().any(|b| matches!(b, b'\r' | b'\n' | 0))
    {
        return Err(IntegrationError::Policy("invalid HTTP header".into()));
    }
    Ok(())
}
