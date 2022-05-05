use ipiis_api_quic::client::IpiisClient;
use ipiis_common::Ipiis;
use ipis::core::account::Account;

#[tokio::test]
async fn test_client() {
    // create an account
    let account = Account::generate();

    // register the environment variables
    ::std::env::set_var("ipis_account_me", account.to_string());

    // try creating a client
    let client = IpiisClient::infer().unwrap();

    // compare the accounts
    assert_eq!(account.to_string(), client.account_me().to_string());
}
