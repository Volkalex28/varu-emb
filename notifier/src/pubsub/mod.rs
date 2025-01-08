use crate::event::{self, traits as __evt};
use crate::rpc::{self, traits as __rpc};
use crate::service::traits as __svc;
use crate::traits::*;
use crate::Metadata;
use core::future::pending;
use core::marker::PhantomData;
use core::ops::{Deref, Index};
use core::sync::atomic::{AtomicUsize, Ordering};
use varuemb_utils::assert::*;

use embassy_sync::channel::TrySendError;
use embassy_time::{Duration, Timer};
use futures_util::FutureExt;
pub use subscriber::{DynSubscription, MixedSubscriber, Subscriber, Subscription};
use varuemb_utils::select;

pub(crate) use private::{PublishConfig, TargetState};

pub mod mixer;
mod subscriber;
pub mod traits;

pub type GetPubSub<P, S> = <S as __svc::Service<<P as traits::PubSub>::Notifier>>::Impl;
pub type GetDynSubscription<N, E> = &'static dyn DynSubscription<GetEvent<N, E>>;
pub type GetEvent<N, E> = event::Event<N, E>;
pub type GetService<P> = <P as traits::PubSub>::Service;
pub type GetEventService<P, E> = <E as __evt::Event<<P as traits::PubSub>::Notifier>>::Service;
pub type GetEventPubSub<P, E> = <GetEventService<P, E> as __svc::Service<<P as traits::PubSub>::Notifier>>::Impl;

pub(crate) mod assert {
    use super::traits;

    pub const fn subscriber<P, E>() -> &'static str
    where
        P: traits::PubSub + 'static,
        E: traits::IsPublisher<P>,
    {
        if crate::is_protected::<P, E>() {
            return "This event can only be accepted by the owning service";
        }
        ""
    }
}

mod private {
    use super::*;

    pub enum TargetState {
        Ok,
        Inactive,
        Filtered,
    }

    pub struct PublishConfig<N, E, Ch, Err, ER = ()>
    where
        N: Notifier,
        E: __evt::Event<N>,
        Err: FnMut(Error<N, E, ER>),
        Ch: for<'s> Fn(&'s subscriber::State, &'static Metadata) -> TargetState,
    {
        pub(crate) data: Option<E>,
        pub(crate) timeout: Option<Duration>,
        pub(crate) checker: Ch,
        pub(crate) error_handler: Err,
        pub(crate) inactive_is_err: bool,
        pub(crate) break_after_error: bool,
        pub(crate) _phantom: PhantomData<*const (N, ER)>,
    }
}

pub enum Error<N, E, RE> {
    Full(event::Event<N, E>),
    Inactive(event::Metadata),
    Timeout(event::Metadata, Duration),
    IncorrectResponse(&'static Metadata, usize),
    Response(&'static Metadata, usize, RE),
}
impl<N, E, RE> Error<N, E, RE> {
    pub fn into_response(self) -> Option<RE> {
        match self {
            Error::Response(_, _, resp) => Some(resp),
            _ => None,
        }
    }
}
impl<N, E: core::fmt::Debug, RE: core::fmt::Debug> core::fmt::Debug for Error<N, E, RE> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Error::Full(f0) => f.debug_tuple("Full").field(&f0).finish(),
            Error::Inactive(f0) => f.debug_tuple("Inactive").field(&f0).finish(),
            Error::Timeout(f0, f1) => f.debug_struct("Timeout").field("Meta", &f0).field("Duration", &f1).finish(),
            Error::IncorrectResponse(f0, f1) => {
                f.debug_struct("IncorrectResponse").field("Meta", &format_args!("{}", f0)).field("Id", &f1).finish()
            }
            Error::Response(f0, f1, err) => f
                .debug_struct("Response")
                .field("Meta", &format_args!("{}", f0))
                .field("Id", &f1)
                .field("Error", &err)
                .finish(),
        }
    }
}

pub struct Container<P: traits::PubSub, const C: usize> {
    pub inner: [PubSub<P>; C],
}

impl<P: traits::PubSub, const C: usize> Container<P, C>
where
    Assert<{ C > 1 }>: IsTrue,
{
    #[inline]
    pub fn get(&self, index: usize) -> Option<&PubSub<P>> {
        let pubsub = self.inner.get(index)?;
        pubsub.index.store(index, Ordering::Relaxed);
        Some(pubsub)
    }
}

impl<P: traits::PubSub + core::marker::Destruct, const C: usize> Container<P, C> {
    pub fn metadata_service(&self) -> &'static Metadata
    where
        P: traits::CanMetadata,
    {
        <P as traits::CanMetadata>::metadata_service()
    }
}

impl<P: traits::PubSub, const C: usize> Container<P, C> {
    pub const fn new() -> Self {
        Self {
            inner: unsafe {
                let mut init = varuemb_utils::ArrayInitializer::new();
                while let Some((i, item)) = init.next() {
                    item.init(PubSub::new(i))
                }
                init.finish()
            },
        }
    }
}

unsafe impl<P: traits::PubSub, const C: usize> traits::GetPubSub<P> for Container<P, C> {
    fn __get(&self, index: usize) -> &self::PubSub<P> {
        let pubsub = &self.inner[index];
        pubsub.index.store(index, Ordering::Relaxed);
        pubsub
    }
}

impl<P: traits::PubSub, const C: usize> Deref for Container<P, C>
where
    Assert<{ C == 1 }>: IsTrue,
{
    type Target = PubSub<P>;
    fn deref(&self) -> &Self::Target {
        &self.inner[0]
    }
}

impl<P: traits::PubSub, const C: usize, I> Index<I> for Container<P, C>
where
    I: core::slice::SliceIndex<[PubSub<P>]>,
    Assert<{ C > 1 }>: IsTrue,
{
    type Output = I::Output;
    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

#[derive(Debug)]
pub struct PublishData {
    pub id: usize,
    pub total: usize,
    pub errors: usize,
    pub published: usize,
    pub not_published: usize,
}

impl PublishData {
    fn new(id: usize, total: usize) -> Self {
        Self { id, total, errors: 0, published: 0, not_published: 0 }
    }
}

pub enum PublishSelector<I: IntoIterator<Item = &'static Metadata>> {
    None,
    Filter(I::IntoIter),
    Target(I::IntoIter),
}

pub struct PublishConfigurator<'p, P, Eh, E = (), I = core::slice::Iter<'static, Metadata>, ER = ()>
where
    P: traits::PubSub,
    Eh: FnMut(Error<P::Notifier, E, ER>),
    I: IntoIterator<Item = &'static Metadata>,
{
    pubsub: &'p PubSub<P>,
    allow_inactive: Option<bool>,
    inactive_is_err: bool,
    break_after_error: bool,
    selector: PublishSelector<I>,
    error_handler: Eh,
    _phantom: PhantomData<*const (E, ER)>,
}

impl<'p, P, E, ER, Eh, I> PublishConfigurator<'p, P, Eh, E, I, ER>
where
    Eh: FnMut(Error<P::Notifier, E, ER>),
    I: IntoIterator<Item = &'static Metadata, IntoIter: Clone>,
    P: traits::PubSub,
{
    pub fn allow_inactive(mut self, allow: bool) -> Self {
        self.allow_inactive = Some(allow);
        self
    }

    pub fn break_after_error(mut self, brk: bool) -> Self {
        self.break_after_error = brk;
        self
    }

    pub fn inactive_is_err(mut self, inactive: bool) -> Self {
        self.inactive_is_err = inactive;
        self
    }

    pub fn set_error_handler<EhNew, ERNew>(self, error_handler: EhNew) -> PublishConfigurator<'p, P, EhNew, E, I, ERNew>
    where
        EhNew: FnMut(Error<P::Notifier, E, ERNew>),
    {
        PublishConfigurator {
            error_handler,
            pubsub: self.pubsub,
            allow_inactive: self.allow_inactive,
            inactive_is_err: self.inactive_is_err,
            break_after_error: self.break_after_error,
            selector: self.selector,
            _phantom: Default::default(),
        }
    }

    pub fn set_filter<INew>(self, filter: INew) -> PublishConfigurator<'p, P, Eh, E, INew, ER>
    where
        INew: IntoIterator<Item = &'static Metadata, IntoIter: Clone>,
    {
        PublishConfigurator {
            selector: PublishSelector::Filter(filter.into_iter()),
            pubsub: self.pubsub,
            allow_inactive: self.allow_inactive,
            inactive_is_err: self.inactive_is_err,
            break_after_error: self.break_after_error,
            error_handler: self.error_handler,
            _phantom: self._phantom,
        }
    }

    pub fn set_targets<INew>(self, targets: INew) -> PublishConfigurator<'p, P, Eh, E, INew, ER>
    where
        INew: IntoIterator<Item = &'static Metadata, IntoIter: Clone>,
    {
        PublishConfigurator {
            selector: PublishSelector::Target(targets.into_iter()),
            pubsub: self.pubsub,
            allow_inactive: self.allow_inactive,
            inactive_is_err: self.inactive_is_err,
            break_after_error: self.break_after_error,
            error_handler: self.error_handler,
            _phantom: self._phantom,
        }
    }

    fn make_config(
        self,
        data: E,
        timeout: Option<Duration>,
    ) -> PublishConfig<P::Notifier, E, impl for<'s> Fn(&'s subscriber::State, &'static Metadata) -> TargetState, Eh, ER>
    where
        E: __evt::Event<P::Notifier, Service = P::Service>,
    {
        let Self { allow_inactive, selector, error_handler, inactive_is_err, break_after_error, .. } = self;
        let allow_inactive = allow_inactive.unwrap_or(true);
        let checker = move |state: &subscriber::State, meta| -> TargetState {
            match &selector {
                PublishSelector::Target(targets) if !targets.clone().any(|target| meta == target) => TargetState::Filtered,
                PublishSelector::Filter(filter) if !filter.clone().all(|target| meta != target) => TargetState::Filtered,
                _ if !allow_inactive && state.receivers.load(Ordering::Acquire) == 0 => TargetState::Inactive,
                _ => TargetState::Ok,
            }
        };
        PublishConfig {
            data: Some(data),
            timeout,
            checker,
            inactive_is_err,
            break_after_error,
            error_handler,
            _phantom: Default::default(),
        }
    }

    pub fn publish(self, data: E) -> PublishData
    where
        E: __evt::Event<P::Notifier, Service = P::Service>,
        PubSub<P>: traits::CanPublishRaw<E, Notifier = P::Notifier>,
    {
        let pubsub = self.pubsub;
        let config = self.make_config(data, None);

        traits::CanPublishRaw::<E>::__raw_publish(pubsub, config)
    }

    pub async fn publish_with(mut self, data: E, timeout: Option<Duration>) -> PublishData
    where
        E: __evt::Event<P::Notifier, Service = P::Service>,
        PubSub<P>: traits::CanPublishRaw<E, Notifier = P::Notifier>,
    {
        let pubsub = self.pubsub;
        if self.allow_inactive.is_none() {
            self.allow_inactive = Some(true)
        }
        let config = self.make_config(data, timeout);

        traits::CanPublishRaw::<E>::__raw_publish_async(pubsub, config).await
    }
}

pub struct PubSub<P: traits::PubSub> {
    pub(crate) inner: P,
    index: AtomicUsize,
    event_id: AtomicUsize,
}

impl<P: traits::PubSub> PubSub<P> {
    pub const fn new(i: usize) -> Self {
        Self { index: AtomicUsize::new(i), inner: P::NEW, event_id: AtomicUsize::new(0) }
    }
}

impl<N, P: traits::PubSub<Notifier = N>> PubSub<P>
where
    N: NotifierService<P::Service>,
{
    pub fn metadata(&self) -> &'static Metadata
    where
        P: traits::CanMetadata,
    {
        P::metadata(self.index.load(Ordering::Relaxed))
    }

    pub fn rpc<S>(&'static self) -> rpc::Container<S::Impl, { S::COUNT }>
    where
        [(); S::COUNT]:,
        N: NotifierService<S>,
        S: __svc::Service<N, Impl: __rpc::Rpc> + __rpc::RpcProvider<N>,
        P: traits::Subscribed<rpc::Response<S::Impl>, Notifier = N> + traits::CanMetadata,
    {
        N::get().__get().rpc(traits::Subscribed::channel(self), self.metadata())
    }

    pub fn subscriber<E>(&'static self) -> Subscriber<P::Notifier, E>
    where
        E: __evt::Event<P::Notifier>,
        Self: traits::Subscribed<E, Notifier = P::Notifier>,
    {
        traits::Subscribed::subscriber(self)
    }

    pub fn subscribers_count<E>(&'static self) -> usize
    where
        E: __evt::Event<<P as traits::PubSub>::Notifier>,
        P: traits::Subscribed<E, Notifier = <P as traits::PubSub>::Notifier>,
    {
        traits::Subscribed::count(self)
    }

    pub fn mixed_subscriber<M>(&'static self) -> MixedSubscriber<P, M>
    where
        P: mixer::SubscriberMixer<M>,
        M: mixer::Mixer<P::Notifier>,
    {
        MixedSubscriber::<P, M>::new(&self.inner)
    }

    pub fn publisher<E>(&self) -> PublishConfigurator<P, impl Fn(Error<P::Notifier, E, ()>), E>
    where
        E: __evt::Event<P::Notifier, Service = P::Service>,
        Self: traits::CanPublish<E, Notifier = P::Notifier>,
    {
        PublishConfigurator {
            pubsub: self,
            allow_inactive: None,
            inactive_is_err: false,
            break_after_error: false,
            selector: PublishSelector::None,
            error_handler: <Self as traits::CanPublish<E>>::error_handler,
            _phantom: Default::default(),
        }
    }

    pub fn publish<E>(&self, data: E) -> PublishData
    where
        E: __evt::Event<P::Notifier, Service = P::Service>,
        Self: traits::CanPublish<E, Notifier = P::Notifier>,
    {
        self.publisher().publish(data)
    }

    pub async fn publish_with<E>(&self, data: E, timeout: Option<Duration>) -> PublishData
    where
        E: __evt::Event<P::Notifier, Service = P::Service>,
        Self: traits::CanPublish<E, Notifier = P::Notifier>,
    {
        self.publisher().publish_with(data, timeout).await
    }

    pub(crate) fn incr_event_id(&self) -> usize {
        self.event_id.fetch_add(1, Ordering::SeqCst)
    }
}

impl<P, N, E> traits::Subscribed<E> for PubSub<P>
where
    N: Notifier,
    E: __evt::Event<N>,
    P: traits::Subscribed<E, Notifier = N> + traits::PubSub<Notifier = N>,
{
    type Notifier = N;
    fn channel(&'static self) -> pubsub::GetSubscriberRet<Self::Notifier, E> {
        self.inner.channel()
    }
    fn subscriber(&'static self) -> self::subscriber::Subscriber<N, E> {
        self.inner.subscriber()
    }
    fn count(&'static self) -> usize {
        self.inner.count()
    }
}

impl<P, E> traits::CanPublish<E> for PubSub<P>
where
    [(); P::Notifier::ID_COUNT]:,
    [(); P::Notifier::CHANNEL_COUNT]:,
    [(); P::Notifier::COUNT_SERVICES]:,
    E: __evt::Event<P::Notifier, Service = P::Service>,
    P: traits::Publisher<E, Notifier: NotifierPublisher<E>> + traits::CanMetadata,
{
    async fn publish_with(&self, data: E, timeout: Option<Duration>) -> PublishData {
        PubSub::publish_with(self, data, timeout).await
    }

    fn publish(&self, data: E) -> PublishData {
        PubSub::publish(self, data)
    }
}

fn pre_publish<P, E, Ch, Eh, ER>(
    pub_sub: &PubSub<P>,
    config: &mut PublishConfig<P::Notifier, E, Ch, Eh, ER>,
) -> ([Result<(GetDynSubscription<P::Notifier, E>, GetEvent<P::Notifier, E>), bool>; P::Notifier::CHANNEL_COUNT], PublishData)
where
    [(); P::Notifier::ID_COUNT]:,
    [(); P::Notifier::CHANNEL_COUNT]:,
    [(); P::Notifier::COUNT_SERVICES]:,
    E: __evt::Event<P::Notifier, Service = P::Service>,
    P: traits::Publisher<E, Notifier: NotifierPublisher<E>> + traits::CanMetadata,
    Eh: FnMut(Error<P::Notifier, E, ER>),
    Ch: for<'s> Fn(&'s subscriber::State, &'static Metadata) -> TargetState,
{
    let (mut event, event_id) = event::Event::new_pubsub(pub_sub, config.data.take().unwrap());
    event.print_pre_publish();

    let mut data = PublishData::new(event_id, P::Notifier::CHANNEL_COUNT);
    let subscribers = crate::subscribers().map(|item| {
        let meta = item.meta();
        let state = item.subscriber.state();
        match (config.checker)(&state, meta) {
            TargetState::Inactive if config.inactive_is_err => {
                data.errors += 1;
                (config.error_handler)(Error::Inactive(event.meta));
                return Err(config.break_after_error);
            }
            TargetState::Filtered | TargetState::Inactive => {
                data.not_published += 1;
                return Err(false);
            }
            TargetState::Ok => (),
        }
        state.sending.store(true, Ordering::Release);

        event.meta.dst = meta;
        let event = event.clone();
        event.print_publish();
        Ok((item.subscriber, event))
    });
    (subscribers, data)
}

fn post_publish<P, E, Ch, Eh, ER>(
    res: Result<(), Error<P::Notifier, E, ER>>,
    config: &mut PublishConfig<P::Notifier, E, Ch, Eh, ER>,
    subscriber: GetDynSubscription<P::Notifier, E>,
    meta: &'static Metadata,
    meta_evt: event::Metadata,
    data: &mut PublishData,
) -> bool
where
    [(); P::Notifier::ID_COUNT]:,
    [(); P::Notifier::CHANNEL_COUNT]:,
    [(); P::Notifier::COUNT_SERVICES]:,
    E: __evt::Event<P::Notifier, Service = P::Service>,
    P: traits::Publisher<E, Notifier: NotifierPublisher<E>> + traits::CanMetadata,
    Eh: FnMut(Error<P::Notifier, E, ER>),
    Ch: for<'s> Fn(&'s subscriber::State, &'static Metadata) -> TargetState,
{
    let mut error = false;
    match res {
        Ok(_) => {
            let is_ok = !matches!((config.checker)(subscriber.state(), meta), TargetState::Ok);
            if is_ok {
                subscriber.clear();
                if config.inactive_is_err {
                    data.errors += 1;
                    error = true;
                    (config.error_handler)(Error::Inactive(meta_evt));
                } else {
                    data.not_published += 1;
                }
            } else {
                data.published += 1;
            }
        }
        Err(err) => {
            error = true;
            data.errors += 1;
            (config.error_handler)(err)
        }
    }
    subscriber.state().sending.store(false, Ordering::Release);

    if error && config.break_after_error {
        return true;
    }
    false
}

impl<P, E> traits::CanPublishRaw<E> for PubSub<P>
where
    [(); P::Notifier::ID_COUNT]:,
    [(); P::Notifier::CHANNEL_COUNT]:,
    [(); P::Notifier::COUNT_SERVICES]:,
    E: __evt::Event<P::Notifier, Service = P::Service>,
    P: traits::Publisher<E, Notifier: NotifierPublisher<E>> + traits::CanMetadata,
{
    type Notifier = P::Notifier;
    fn __raw_publish<Ch, Eh, ER>(&self, mut config: PublishConfig<Self::Notifier, E, Ch, Eh, ER>) -> PublishData
    where
        E: __evt::Event<Self::Notifier>,
        Eh: FnMut(Error<Self::Notifier, E, ER>),
        Ch: for<'s> Fn(&'s subscriber::State, &'static Metadata) -> TargetState,
    {
        let (subscribers, mut data) = pre_publish(self, &mut config);

        for item in subscribers {
            let (subscriber, event) = match item {
                Ok(event) => event,
                Err(true) => break,
                Err(false) => continue,
            };

            let meta = event.meta.dst;
            let meta_evt = event.meta;
            let res = subscriber.sender().try_send(event).map_err(|TrySendError::Full(evt)| Error::Full(evt));

            if post_publish(res, &mut config, subscriber, meta, meta_evt, &mut data) {
                break;
            }
        }
        data
    }

    async fn __raw_publish_async<Ch, Eh, ER>(&self, mut config: PublishConfig<Self::Notifier, E, Ch, Eh, ER>) -> PublishData
    where
        E: __evt::Event<Self::Notifier>,
        Eh: FnMut(Error<Self::Notifier, E, ER>),
        Ch: for<'s> Fn(&'s subscriber::State, &'static Metadata) -> TargetState,
    {
        let (subscribers, mut data) = pre_publish(self, &mut config);

        for item in subscribers {
            let (subscriber, event) = match item {
                Ok(event) => event,
                Err(true) => break,
                Err(false) => continue,
            };

            let meta = event.meta.dst;
            let meta_evt = event.meta;
            let timer = if let Some(timeout) = config.timeout {
                Timer::after(timeout).map(move |_| timeout).left_future()
            } else {
                pending().right_future()
            };
            let res = select! {
                _send = subscriber.sender().send(event) => { Ok(()) }
                timeout = timer => { Err(Error::Timeout(meta_evt, timeout)) }
            };

            if post_publish(res, &mut config, subscriber, meta, meta_evt, &mut data) {
                break;
            }
        }
        data
    }
}
