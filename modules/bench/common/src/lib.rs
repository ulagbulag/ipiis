use std::time::{Duration, Instant};

use ipiis_common::{define_io, external_call, Ipiis, ServerResult};
use ipis::{
    async_trait::async_trait,
    core::{
        account::{GuaranteeSigned, GuarantorSigned, Verifier},
        anyhow::Result,
    },
    stream::DynStream,
};

#[async_trait]
pub trait IpiisBench {
    async fn ping(&self, data: DynStream<'static, Vec<u8>>) -> Result<Duration>;
}

#[async_trait]
impl<IpiisClient> IpiisBench for IpiisClient
where
    IpiisClient: Ipiis + Send + Sync,
{
    async fn ping(&self, data: DynStream<'static, Vec<u8>>) -> Result<Duration> {
        // begin measuring time
        let instant = Instant::now();

        // next target
        let target = self.get_account_primary(KIND.as_ref()).await?;

        // external call
        let mut recv = external_call!(
            client: self,
            target: KIND.as_ref() => &target,
            request: crate::io => Ping,
            sign: self.sign(target, ())?,
            inputs: {
                data: data,
            },
            inputs_mode: none,
            outputs: send,
        );

        // recv sign
        let sign: GuarantorSigned<()> = DynStream::recv(&mut recv).await?.into_owned().await?;

        // verify sign
        let _ = sign.verify(Some(target))?;

        // recv data
        let _ = DynStream::<Vec<u8>>::recv(recv).await?;

        // finish measuring time
        Ok(instant.elapsed())
    }
}

define_io! {
    Ping {
        inputs: {
            data: Vec<u8>,
        },
        input_sign: GuaranteeSigned<()>,
        outputs: {
            data: Vec<u8>,
        },
        output_sign: GuarantorSigned<()>,
        generics: { },
    },
}

::ipis::lazy_static::lazy_static! {
    pub static ref KIND: Option<::ipis::core::value::hash::Hash> = Some(
        ::ipis::core::value::hash::Hash::with_str("__ipis__ipiis__bench__"),
    );
}
