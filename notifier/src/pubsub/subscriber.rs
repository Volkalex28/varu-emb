use super::{__evt, event, mixer, traits};
use core::future::pending;
use core::sync::atomic::Ordering::*;
use core::sync::atomic::{AtomicBool, AtomicUsize};
use embassy_sync::blocking_mutex::raw;
use embassy_sync::channel;

type RawMutex = raw::CriticalSectionRawMutex;

pub struct State {
    pub(crate) receivers: AtomicUsize,
    pub(crate) sending: AtomicBool,
}

pub struct Subscription<P, E, const C: usize>
where
    P: traits::PubSub,
    E: __evt::Event<P::Notifier>,
{
    inner: channel::Channel<RawMutex, event::Event<P::Notifier, E>, C>,
    state: State,
}

impl<P, E, const C: usize> Subscription<P, E, C>
where
    P: traits::PubSub,
    E: traits::IsPublisher<P>,
    varuemb_utils::assert::Msg<{ super::assert::subscriber::<P, E>() }>: varuemb_utils::assert::IsTrue,
{
    pub const fn default() -> Self {
        Self::new()
    }
}

impl<P, E, const C: usize> Subscription<P, E, C>
where
    P: traits::PubSub,
    E: __evt::Event<P::Notifier>,
{
    pub(crate) const fn new() -> Self {
        Self {
            inner: channel::Channel::new(),
            state: State { receivers: AtomicUsize::new(0), sending: AtomicBool::new(false) },
        }
    }

    pub(crate) fn as_dyn(&'static self) -> &'static dyn DynSubscription<event::Event<P::Notifier, E>> {
        self
    }
}

use super::traits::GetSubscriberRet;
impl<P, E, const C: usize> From<&'static Subscription<P, E, C>> for GetSubscriberRet<P::Notifier, E>
where
    P: traits::PubSub,
    E: __evt::Event<P::Notifier>,
{
    fn from(ch: &'static Subscription<P, E, C>) -> Self {
        ch.as_dyn()
    }
}

pub trait DynSubscription<E: 'static> {
    fn sender(&'static self) -> channel::DynamicSender<'static, E>;
    fn receiver(&'static self) -> channel::DynamicReceiver<'static, E>;
    fn state(&'static self) -> &'static State;
    fn clear(&'static self) {
        while self.receiver().try_receive().is_ok() {}
    }
}

impl<P, E, const C: usize> DynSubscription<event::Event<P::Notifier, E>> for Subscription<P, E, C>
where
    P: traits::PubSub,
    E: __evt::Event<P::Notifier>,
{
    fn sender(&'static self) -> channel::DynamicSender<'static, event::Event<P::Notifier, E>> {
        self.inner.sender().into()
    }

    fn receiver(&'static self) -> channel::DynamicReceiver<'static, event::Event<P::Notifier, E>> {
        self.inner.receiver().into()
    }

    fn state(&'static self) -> &'static State {
        &self.state
    }
}

pub struct Subscriber<N, E>
where
    E: __evt::Event<N>,
    N: crate::traits::Notifier,
{
    pub(crate) state: bool,
    pub(crate) channel: &'static dyn DynSubscription<event::Event<N, E>>,
}

impl<N, E> Subscriber<N, E>
where
    E: __evt::Event<N>,
    N: crate::traits::Notifier,
{
    pub(crate) fn new(channel: &'static dyn DynSubscription<event::Event<N, E>>) -> Self {
        channel.state().receivers.fetch_add(1, AcqRel);
        Self { channel, state: true }
    }

    pub fn try_next(&mut self) -> Option<event::Event<N, E>> {
        if !self.state {
            return None;
        }
        self.channel.receiver().try_receive().ok()
    }

    pub async fn next(&mut self) -> event::Event<N, E> {
        if !self.state {
            return pending().await;
        }
        self.channel.receiver().receive().await
    }

    pub fn try_next_raw(&mut self) -> Option<E> {
        self.try_next().map(|e| e.data)
    }

    pub async fn next_raw(&mut self) -> E {
        self.next().await.data
    }

    pub fn state(&self) -> bool {
        self.state
    }

    pub fn set_state(&mut self, state: bool) {
        if state != self.state {
            if state {
                self.channel.state().receivers.fetch_add(1, AcqRel);
            } else {
                self.__drop()
            }
        }
        self.state = state;
    }

    pub fn clear(&mut self) {
        self.channel.clear()
    }

    fn __drop(&mut self) {
        let state = self.channel.state();
        if state.receivers.fetch_sub(1, AcqRel) == 1 {
            self.clear()
        }
    }
}

impl<N, E> Clone for Subscriber<N, E>
where
    E: __evt::Event<N>,
    N: crate::traits::Notifier,
{
    fn clone(&self) -> Self {
        Self::new(self.channel)
    }
}

impl<N, E> Drop for Subscriber<N, E>
where
    E: __evt::Event<N>,
    N: crate::traits::Notifier,
{
    fn drop(&mut self) {
        self.__drop()
    }
}

pub struct MixedSubscriber<P: 'static, M>
where
    P: mixer::SubscriberMixer<M>,
    M: mixer::Mixer<P::Notifier>,
{
    inner: P::Mixed,
}

impl<P, M> MixedSubscriber<P, M>
where
    P: mixer::SubscriberMixer<M>,
    M: mixer::Mixer<P::Notifier>,
{
    pub(crate) fn new(inner: &'static P) -> Self {
        Self { inner: <P as mixer::SubscriberMixer<M>>::__new_mixed(inner) }
    }

    pub async fn next(&mut self) -> event::Event<P::Notifier, M> {
        <Self as traits::SubscribedMixed<M>>::next(self).await
    }

    pub fn try_next(&mut self) -> Option<event::Event<P::Notifier, M>> {
        <Self as traits::SubscribedMixed<M>>::try_next(self)
    }
}

impl<P, M> traits::SubscribedMixed<M> for MixedSubscriber<P, M>
where
    P: mixer::SubscriberMixer<M>,
    M: mixer::Mixer<P::Notifier>,
{
    type Subscriber = P;
    type Notifier = P::Notifier;

    async fn next(&mut self) -> event::Event<Self::Notifier, M> {
        P::__mixed(&mut self.inner).await
    }

    fn try_next(&mut self) -> Option<event::Event<Self::Notifier, M>> {
        P::__try_mixed(&mut self.inner)
    }
}
