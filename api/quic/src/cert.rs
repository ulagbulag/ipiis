use std::{sync::Arc, time::SystemTime};

use ipis::core::{
    account::{Account, AccountRef},
    anyhow::{anyhow, Result},
    ed25519_dalek::ed25519::{pkcs8::EncodePrivateKey, KeypairBytes},
};
use rustls::{
    client::{ServerCertVerified, ServerCertVerifier},
    Certificate, Error, PrivateKey, ServerName,
};

pub fn get_name(account: &AccountRef) -> String {
    let account = account.to_string();
    format!("{account}.ipiis")
}

pub(crate) fn generate(account: &Account) -> Result<(PrivateKey, Vec<Certificate>)> {
    let keypair = KeypairBytes::from_bytes(&account.to_bytes())
        .to_pkcs8_der()
        .map_err(|_| anyhow!("failed to convert keypair to DER-encoded ASN.1"))?;

    let mut keypair = keypair.as_ref().to_vec();
    keypair[1] = 83;
    keypair[48] = 3;
    keypair.insert(48, 35);
    keypair.insert(48, 161);

    let mut params = ::rcgen::CertificateParams::new(vec![get_name(&account.account_ref())]);
    params.alg = &::rcgen::PKCS_ED25519;
    params.key_pair = Some(::rcgen::KeyPair::from_der(&keypair).unwrap());

    let cert = rcgen::Certificate::from_params(params).unwrap();
    let cert_der = cert.serialize_der().unwrap();
    let priv_key = cert.serialize_private_key_der();
    let priv_key = ::rustls::PrivateKey(priv_key);
    let cert_chain = vec![::rustls::Certificate(cert_der)];
    Ok((priv_key, cert_chain))
}

/// Dummy certificate verifier that treats any certificate as valid.
/// FIXME: such verification is vulnerable to MITM attacks, but convenient for testing.
pub(crate) struct ServerVerification;

impl ServerVerification {
    pub(crate) fn new() -> Arc<Self> {
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
