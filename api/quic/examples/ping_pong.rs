use std::sync::Arc;

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
        account::{Account, AccountRef, GuaranteeSigned},
        anyhow::Result,
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
    let req = Arc::new(Request {
        name: "Alice".to_string(),
        age: 42,
    });

    for _ in 0..5 {
        // recv data
        let res: GuaranteeSigned<String> = client
            .call_permanent_deserialized(Opcode::TEXT, &server, req.clone())
            .await?;

        // verify data
        assert_eq!(
            res.data.data,
            format!("hello, {} years old {}!", &req.name, req.age),
        );
    }
    Ok(())
}

async fn run_client(server: AccountRef, certs: &[Certificate], port: u16) -> Result<IpiisClient> {
    // generate an account
    let account = Account::generate();

    // init a client
    let client = IpiisClient::new(account, None, certs)?;
    client.add_address(server, format!("127.0.0.1:{}", port).parse()?)?;
    Ok(client)
}

async fn run_server(port: u16) -> Result<(AccountRef, Vec<Certificate>)> {
    // generate an account
    let account = Account::generate();
    let public_key = AccountRef {
        public_key: account.public_key(),
    };

    // init a server
    let server = IpiisServer::new(account, None, &[], port)?;
    let certs = server.get_cert_chain()?;

    // accept a single connection
    tokio::spawn(async move { server.run(handle).await });

    Ok((public_key, certs))
}

#[derive(Class, Clone, Debug, PartialEq, Archive, Serialize, Deserialize)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(CheckBytes, Debug, PartialEq))]
pub struct Request {
    name: String,
    age: u32,
}

async fn handle(req: Pinned<GuaranteeSigned<Arc<Request>>>) -> Result<String> {
    // resolve data
    let req = &req.data.data;

    // handle data
    let res = format!("hello, {} years old {}!", &req.name, req.age);

    Ok(res)
}
