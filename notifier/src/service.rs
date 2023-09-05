use varuemb_utils::ConstDefault;

use crate::{
    pubsub::{self, traits as __pub},
    rpc::{self, traits as __rpc},
    traits as __traits,
};

pub struct Service<N, S>
where
    N: __traits::Notifier,
    S: traits::Service<N>,
    [(); S::COUNT]:,
{
    pub(crate) pubsub: pubsub::Container<S::Impl, { S::COUNT }>,
}
unsafe impl<N, S> Sync for Service<N, S>
where
    N: __traits::Notifier,
    S: traits::Service<N>,
    [(); S::COUNT]:,
{
}

impl<N, S> const ConstDefault for Service<N, S>
where
    N: __traits::Notifier,
    S: traits::Service<N>,
    [(); S::COUNT]:,
{
    fn default() -> Self {
        Self {
            pubsub: pubsub::Container::<S::Impl, { S::COUNT }>::new(),
        }
    }
}

impl<N, S> Service<N, S>
where
    N: __traits::NotifierService<S>,
    S: traits::Service<N>,
    [(); S::COUNT]:,
{
    pub const COUNT: usize = S::COUNT;

    pub fn id(&self) -> usize {
        N::ID
    }

    pub fn pubsub(&self) -> &pubsub::Container<S::Impl, { S::COUNT }> {
        &self.pubsub
    }

    pub fn rpc(
        &self,
        channel: __pub::GetSubscriberRet<N, rpc::Response<S::Impl>>,
        meta: &'static crate::Metadata,
    ) -> rpc::Container<S::Impl, { S::COUNT }>
    where
        S::Impl: __rpc::Rpc,
        S: __rpc::RpcProvider<N>,
    {
        rpc::Container::new(channel, meta)
    }
}

unsafe impl<N, S> __pub::GetPubSub<S::Impl> for Service<N, S>
where
    N: __traits::Notifier,
    S: traits::Service<N>,
    [(); S::COUNT]:,
{
    fn __get(&self, index: usize) -> &pubsub::PubSub<S::Impl> {
        __pub::GetPubSub::__get(&self.pubsub, index)
    }
}

pub mod traits {
    use crate::{
        assert::*,
        pubsub::{self, traits as __pub},
        rpc::{self, traits as __rpc},
        traits as __traits,
    };

    pub trait Service<N: __traits::Notifier>: Sized {
        const COUNT: usize;
        type Impl: ~const __pub::PubSub<Notifier = N, Service = Self>;

        fn notif() -> &'static pubsub::Container<Self::Impl, { Self::COUNT }>
        where
            [(); Self::COUNT]:,
            N: __traits::NotifierService<Self>,
        {
            &N::get().__get().pubsub()
        }

        async fn rpc_request(
            subscriber: &mut pubsub::Subscriber<N, rpc::Request<Self::Impl>>,
        ) -> rpc::RpcRequest<Self::Impl>
        where
            [(); Self::COUNT]:,
            N: __traits::NotifierService<Self>,
            Self: __rpc::RpcProvider<N> + 'static,
            for<'r> &'r Self::Request: Into<usize>,
            Self::Impl: __rpc::Rpc<Notifier = N, Service = Self> + __pub::CanMetadata,
        {
            rpc::Rpc::<Self::Impl>::request(subscriber).await
        }
    }

    #[const_trait]
    pub trait ServiceMetadata<N: __traits::NotifierService<Self>>: Service<N>
    where
        Assert<{ crate::metadata::check::<N, Self>() }>: True,
    {
        const META_SERVICE: crate::Metadata;
        const META: &'static [crate::Metadata];
    }
    varuemb_utils::const_wrapper! {
        use varuemb_utils::ArrayInitializer as Init;
        use crate::{metadata::check, Metadata};
        impl<S: Service<N>, N: __traits::NotifierService<S>> const ServiceMetadata<N> for S
        where
            [(); Self::COUNT]:,
            Assert<{ check::<N, Self>() }>: True,
        {
            const META_SERVICE: crate::Metadata = crate::Metadata::new_service::<N, Self>();
            const META: &'static [crate::Metadata] = &Init::init::<{ Self::COUNT }, _, _>(&Metadata::new::<N, Self>);
        }
    }
}
