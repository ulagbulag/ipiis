use bytecheck::CheckBytes;
use ipiis_api_quic::{client::IpiisClient, opcode::Opcode, server::IpiisServer};
use ipiis_common::Ipiis;
use ipis::{
    class::Class,
    core::{
        account::{Account, AccountRef},
        anyhow::{anyhow, Result},
        signature::Keypair,
    },
};
use rkyv::{Archive, Deserialize, Infallible, Serialize};
use rustls::Certificate;

#[tokio::main]
async fn main() -> Result<()> {
    // init peers
    let (server, certs) = run_server(5001).await?;
    let client = run_client(server, &certs, 5001).await?;

    // create a data
    let req = Request {
        name: "Alice".to_string(),
        age: 42,
    };

    for _ in 0..5 {
        // recv data
        let res: String = client
            .call_deserialized(Opcode::TEXT, &server, &req, &mut Infallible)
            .await?;

        // verify data
        assert_eq!(res, format!("hello, {} years old {}!", &req.name, req.age));
    }
    Ok(())
}

async fn run_client(server: AccountRef, certs: &[Certificate], port: u16) -> Result<IpiisClient> {
    // generate keypair
    let keypair = Keypair::generate();

    // init a client
    let client = IpiisClient::new(Account { keypair }, None, certs)?;
    client.add_address(server, format!("127.0.0.1:{}", port).parse()?)?;
    Ok(client)
}

async fn run_server(port: u16) -> Result<(AccountRef, Vec<Certificate>)> {
    // generate keypair
    let keypair = Keypair::generate();
    let public_key = AccountRef {
        public_key: keypair.public_key(),
    };

    // init a server
    let server = IpiisServer::new(Account { keypair }, None, &[], port)?;
    let certs = server.get_cert_chain()?;

    // accept a single connection
    tokio::spawn(server.run::<Request, _, _, _>(handle));

    Ok((public_key, certs))
}

#[derive(Class, Debug, PartialEq, Archive, Serialize, Deserialize)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(CheckBytes, Debug, PartialEq))]
pub struct Request {
    name: String,
    age: u32,
}

async fn handle(bytes: Vec<u8>) -> Result<String> {
    // unpack data
    let req = ::ipis::rkyv::check_archived_root::<Request>(&bytes)
        .map_err(|_| anyhow!("failed to parse the received bytes"))?;

    // handle data
    let res = format!("hello, {} years old {}!", &req.name, req.age);

    Ok(res)
}
