use std::{sync::Arc, time::SystemTime};

use rustls::{
    client::{ServerCertVerified, ServerCertVerifier},
    Certificate, Error, ServerName,
};

/// Dummy certificate verifier that treats any certificate as valid.
/// FIXME: such verification is vulnerable to MITM attacks, but convenient for testing.
pub struct ServerVerification;

impl ServerVerification {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl ServerCertVerifier for ServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &Certificate,
        _intermediates: &[Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: SystemTime,
    ) -> Result<ServerCertVerified, Error> {
        Ok(ServerCertVerified::assertion())
    }
}
