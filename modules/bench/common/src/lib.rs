use ipiis_common::{define_io, external_call, Ipiis, ServerResult};
use ipis::{
    async_trait::async_trait,
    core::{
        account::{GuaranteeSigned, GuarantorSigned},
        anyhow::Result,
        data::Data,
    },
    stream::DynStream,
};

#[async_trait]
pub trait IpiisBench {
    async fn ping(&self, data: DynStream<'static, Vec<u8>>) -> Result<()>;
}

#[async_trait]
impl<IpiisClient> IpiisBench for IpiisClient
where
    IpiisClient: Ipiis + Send + Sync,
{
    async fn ping(&self, data: DynStream<'static, Vec<u8>>) -> Result<()> {
        // next target
        let target = self.get_account_primary(KIND.as_ref()).await?;

        // external call
        external_call!(
            client: self,
            target: KIND.as_ref() => &target,
            request: crate::io => Ping,
            sign: self.sign_owned(target, 42)?,
            inputs: {
                data: data,
            },
            inputs_mode: none,
            outputs: { },
        );

        // unpack data
        Ok(())
    }
}

define_io! {
    Ping {
        inputs: {
            data: Vec<u8>,
        },
        input_sign: Data<GuaranteeSigned, u8>,
        outputs: { },
        output_sign: Data<GuarantorSigned, u8>,
        generics: { },
    },
}

::ipis::lazy_static::lazy_static! {
    pub static ref KIND: Option<::ipis::core::value::hash::Hash> = Some(
        ::ipis::core::value::hash::Hash::with_str("__ipis__ipiis__bench__"),
    );
}
