use bytecheck::CheckBytes;
use ipiis_api_quic::{
    client::IpiisClient,
    common::{opcode::Opcode, Ipiis},
    rustls::Certificate,
    server::IpiisServer,
};
use ipis::{
    class::Class,
    core::{
        account::{Account, AccountRef},
        anyhow::Result,
        signature::Keypair,
    },
    pin::Pinned,
};
use rkyv::{Archive, Deserialize, Serialize};

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
            .call_deserialized(Opcode::TEXT, &server, &req)
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

async fn handle(req: Pinned<Request>) -> Result<String> {
    // handle data
    let res = format!("hello, {} years old {}!", &req.name, req.age);

    Ok(res)
}
