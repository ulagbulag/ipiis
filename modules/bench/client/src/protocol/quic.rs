use ipiis_api_quic::client::IpiisClient;
use ipiis_common::Ipiis;
use ipiis_modules_bench_common::{args, KIND};
use ipis::{
    async_trait::async_trait,
    core::anyhow::{Ok, Result},
    env::Infer,
};

pub struct ProtocolImpl {
    client: IpiisClient,
}

impl ProtocolImpl {
    pub async fn try_new(ipiis: &args::ArgsIpiis) -> Result<Self> {
        // init client
        let client = IpiisClient::try_infer().await?;

        // register the server account as primary
        client
            .set_account_primary(KIND.as_ref(), &ipiis.account)
            .await?;
        client
            .set_address(KIND.as_ref(), &ipiis.account, &ipiis.address)
            .await?;

        Ok(Self { client })
    }
}

#[async_trait]
impl super::Protocol for ProtocolImpl {
    async fn to_string(&self) -> Result<String> {
        Ok("quic".into())
    }

    async fn ping(&self, ctx: super::BenchmarkCtx) -> Result<()> {
        super::ping(&self.client, ctx).await
    }
}
