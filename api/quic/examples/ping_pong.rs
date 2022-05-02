use ipiis_api_quic::{client::IpiisClient, opcode::Opcode, server::IpiisServer};
use ipiis_common::Ipiis;
use ipis::core::{
    account::{Account, AccountRef},
    anyhow::Result,
    signature::Keypair,
};
use rkyv::Infallible;
use rustls::Certificate;

#[tokio::main]
async fn main() -> Result<()> {
    let (server, certs) = run_server(5001).await?;
    let client = run_client(server, &certs, 5001).await?;

    let res: String = client
        .call_deserialized(Opcode::TEXT, &server, &258i32, &mut Infallible)
        .await?;
    assert_eq!(res, "hello world!");
    Ok(())
}

async fn run_client(server: AccountRef, certs: &[Certificate], port: u16) -> Result<IpiisClient> {
    let keypair = Keypair::generate();
    let public_key = AccountRef {
        public_key: keypair.public_key(),
    };

    // init a client
    let client = IpiisClient::new(Account { keypair }, None, certs)?;
    client.add_address(server, format!("127.0.0.1:{}", port).parse()?)?;
    Ok(client)
}

async fn run_server(port: u16) -> Result<(AccountRef, Vec<Certificate>)> {
    let keypair = Keypair::generate();
    let public_key = AccountRef {
        public_key: keypair.public_key(),
    };

    // accept a single connection
    let server = IpiisServer::new(Account { keypair }, None, &[], port)?;
    let certs = server.get_cert_chain()?;
    tokio::spawn(server.run());

    Ok((public_key, certs))
}
