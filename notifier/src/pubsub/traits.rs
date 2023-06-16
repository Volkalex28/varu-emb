use crate::{
    assert::*,
    event,
    pubsub::{__evt, mixer},
    service::traits as __svc,
    traits::*,
};
use embassy_time::Duration;

#[const_trait]
pub trait PubSub: Sized + 'static {
    type Notifier: Notifier;
    type Service: __svc::Service<Self::Notifier, Impl = Self>;

    fn __new() -> Self;
}
pub trait Publisher<E>: PubSub {
    const PROTECTED: bool = false;
}

pub type GetSubscriberRet<N, E> = super::GetDynSubscription<N, E>;
pub trait Subscriber<E: IsPublisher<Self>>: PubSub {
    const IMPL: bool;
    fn __get(&'static self) -> GetSubscriberRet<Self::Notifier, E>;
}
impl<P, E> Subscriber<E> for P
where
    P: PubSub + 'static,
    E: IsPublisher<P>,
{
    default const IMPL: bool = false;
    default fn __get(&self) -> GetSubscriberRet<P::Notifier, E> {
        unreachable!()
    }
}

pub(crate) unsafe trait GetPubSub<P: PubSub> {
    fn __get(&self, index: usize) -> &super::PubSub<P>;
}

pub trait Subscribed<E: __evt::Event<Self::Notifier>> {
    type Notifier: Notifier;

    fn channel(&'static self) -> GetSubscriberRet<Self::Notifier, E>;
    fn subscriber(&'static self) -> super::subscriber::Subscriber<Self::Notifier, E>;
    fn count(&'static self) -> usize;
}
impl<N, E, P> Subscribed<E> for P
where
    N: NotifierServiceEvent<E>,
    E: __evt::Event<N> + IsPublisher<P>,
    P: PubSub<Notifier = N, Service: IsSubscribed<N, E>>,
    Assert<{ crate::is_pubsub_impl::<P::Service, N, E>() }>: True,
{
    type Notifier = N;
    fn channel(&'static self) -> GetSubscriberRet<N, E> {
        self.__get()
    }
    fn subscriber(&'static self) -> super::subscriber::Subscriber<N, E> {
        super::subscriber::Subscriber::new(self.channel())
    }
    fn count(&'static self) -> usize {
        self.channel().state().lock().unwrap().receivers
    }
}

pub trait SubscribedMixed<M: mixer::Mixer<Self::Notifier>> {
    type Subscriber: mixer::SubscriberMixer<M, Notifier = Self::Notifier>;
    type Notifier: Notifier;

    async fn next(&mut self) -> event::Event<Self::Notifier, M>;
    fn try_next(&mut self) -> Option<event::Event<Self::Notifier, M>>;
}

pub trait CanMetadata {
    fn metadata_service() -> &'static crate::Metadata;
    fn metadata(index: usize) -> &'static crate::Metadata;
}
impl<N, P> CanMetadata for P
where
    N: NotifierService<P::Service>,
    P: PubSub<Notifier = N, Service: __svc::ServiceMetadata<N>>,
    Assert<{ crate::metadata::check::<N, P::Service>() }>: True,
    [(); <P::Service as __svc::Service<N>>::COUNT]:,
{
    fn metadata_service() -> &'static crate::Metadata {
        &<P::Service as __svc::ServiceMetadata<N>>::META_SERVICE
    }
    fn metadata(index: usize) -> &'static crate::Metadata {
        &<P::Service as __svc::ServiceMetadata<N>>::META[index]
    }
}

pub trait CanPublish<E: __evt::Event<Self::Notifier>>: CanPublishRaw<E> {
    fn publish(&self, data: E) -> super::PublishData;
    async fn publish_with(&self, data: E, timeout: Option<Duration>) -> super::PublishData;

    fn error_handler<ER: core::fmt::Debug>(err: super::Error<Self::Notifier, E, ER>) {
        Self::print_error(&err)
    }

    fn print_error<ER: core::fmt::Debug>(err: &super::Error<Self::Notifier, E, ER>) {
        match err {
            super::Error::Timeout(meta, timeout) => meta.print_publish_err_timeout(*timeout),
            super::Error::Inactive(meta) => meta.print_publish_err_inactive(),
            super::Error::Full(event) => event.print_publish_err_full(),
            super::Error::IncorrectResponse(meta, id) => {
                log::error!(
                    target: &format!("Pub<{}>", meta.name()),
                    "Service {} expects another response on request {}",
                    meta.name(),
                    id
                )
            }
            super::Error::Response(meta, id, err) => log::error!(
                target: &format!("Pub<{}>", meta.name()),
                "Service {} got error on request {}: {:?}",
                meta.name(),
                id,
                err
            ),
        }
    }
}

pub(crate) use private::CanPublishRaw;
mod private {
    use super::*;
    use crate::pubsub as root;

    pub trait CanPublishRaw<E: __evt::Event<Self::Notifier>> {
        type Notifier: Notifier;

        fn __raw_publish<Ch, Eh, ER>(
            &self,
            config: root::PublishConfig<Self::Notifier, E, Ch, Eh, ER>,
        ) -> root::PublishData
        where
            E: __evt::Event<Self::Notifier>,
            Eh: FnMut(root::Error<Self::Notifier, E, ER>) -> (),
            Ch: for<'s> Fn(
                &'s root::subscriber::State,
                &'static root::Metadata,
            ) -> root::TargetState;
        async fn __raw_publish_async<Ch, Eh, ER>(
            &self,
            config: root::PublishConfig<Self::Notifier, E, Ch, Eh, ER>,
        ) -> root::PublishData
        where
            E: __evt::Event<Self::Notifier>,
            Eh: FnMut(root::Error<Self::Notifier, E, ER>) -> (),
            Ch: for<'s> Fn(
                &'s root::subscriber::State,
                &'static root::Metadata,
            ) -> root::TargetState;
    }
}

pub trait IsPublisher<P: PubSub>: __evt::Event<P::Notifier> {
    type Publisher: Publisher<Self>;
}
impl<P, E> IsPublisher<P> for E
where
    P: PubSub<Notifier: NotifierServiceEvent<E>>,
    E: __evt::Event<P::Notifier, Service: __svc::Service<P::Notifier, Impl: Publisher<E>>>,
{
    type Publisher = super::GetEventPubSub<P, E>;
}

pub trait IsSubscribed<N, E> {
    const IMPL: bool;
    const COUNT: usize;
}
impl<N, S, E> IsSubscribed<N, E> for S
where
    N: NotifierService<S>,
    E: IsPublisher<S::Impl>,
    S: __svc::Service<N, Impl: Subscriber<E>>,
{
    const IMPL: bool = S::Impl::IMPL;
    const COUNT: usize = S::COUNT;
}
