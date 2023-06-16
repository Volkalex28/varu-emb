use crate::{
    event::traits as __evt, pubsub::traits as __pubsub, service::traits as __svc, traits::*,
};

pub trait Rpc: __pubsub::PubSub
where
    Self::Service: RpcProvider<Self::Notifier>,
{
    fn __new_rpc(rpc: super::Rpc<Self>) -> GetRpc<Self> {
        <Self::Service as RpcProvider<Self::Notifier>>::__new_rpc(rpc)
    }
}

impl<P: __pubsub::PubSub> Rpc for P where Self::Service: RpcProvider<Self::Notifier> {}
impl<R: Rpc> __pubsub::Publisher<super::GetResponse<Self, R::Service>> for R where
    Self::Service: RpcProvider<Self::Notifier>
{
}
impl<R: Rpc> __pubsub::Publisher<super::GetRequest<Self, R::Service>> for R
where
    Self::Service: RpcProvider<Self::Notifier>,
{
    const PROTECTED: bool = true;
}

pub type GetSubscriberRet<N, S> =
    __pubsub::GetSubscriberRet<N, super::Response<<S as __svc::Service<N>>::Impl>>;
pub type GetRpc<R: Rpc> = <R::Service as RpcProvider<R::Notifier>>::Rpc;

pub trait RpcProvider<N: Notifier>: __svc::Service<N, Impl: Rpc> {
    type Rpc;
    type Request: __evt::Event<N, Service = Self>;
    type Response: __evt::Event<N, Service = Self>;
    type Error: core::fmt::Debug + Clone + 'static;

    fn __new_rpc(rpc: super::Rpc<Self::Impl>) -> Self::Rpc;
}

default impl<N: Notifier, S: __svc::Service<N, Impl: Rpc>> RpcProvider<N> for S {
    type Error = ();
}
