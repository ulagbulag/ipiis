use ipiis_api::{client::IpiisClient, common::Ipiis};
use ipis::{core::account::Account, env::Infer, tokio};

#[tokio::test]
async fn test_client() {
    // create an account
    let account = Account::generate();

    // register the environment variables
    ::std::env::set_var("ipis_account_me", account.to_string());

    // try creating a client
    let client = IpiisClient::infer().await;

    // compare the accounts
    assert_eq!(
        account.account_ref().to_string(),
        client.account_ref().to_string(),
    );
}
