use core::marker::PhantomData;
use embassy_time::Duration;

pub struct Event<N, E> {
    pub(crate) data: E,
    pub(crate) meta: Metadata,
    pub(crate) _phantom: PhantomData<*const N>,
}

unsafe impl<N, E: Send> Send for Event<N, E> {}

impl<N, E: core::fmt::Debug> core::fmt::Debug for Event<N, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Event").field("data", &self.data).finish()
    }
}

impl<N: crate::traits::Notifier, E> Event<N, E> {
    pub(crate) fn new_pubsub<P>(pubsub: &crate::pubsub::PubSub<P>, data: E) -> (Self, usize)
    where
        E: traits::Event<N>,
        P: crate::pubsub::traits::PubSub<Notifier = N> + crate::pubsub::traits::CanMetadata,
        P::Notifier: crate::traits::NotifierService<P::Service>,
    {
        let event_id = pubsub.incr_event_id();
        let event = Self {
            data,
            meta: Metadata {
                id: event_id,
                src: pubsub.metadata(),
                dst: pubsub.metadata(),
            },
            _phantom: Default::default(),
        };
        (event, event_id)
    }

    pub fn data(self) -> E {
        self.data
    }

    pub fn meta(&self) -> Metadata {
        self.meta
    }

    pub fn map<M>(self, mapper: impl FnOnce(E) -> M) -> Event<N, M> {
        Event {
            data: (mapper)(self.data),
            meta: self.meta,
            _phantom: Default::default(),
        }
    }

    pub(crate) fn print_pre_publish(&self)
    where
        E: traits::Event<N>,
    {
        let target = self.meta.log_target();
        log::info!(
            target: &target,
            "Publishing<id: {}>: {:?}",
            self.meta.id,
            self.data
        )
    }

    pub(crate) fn print_publish(&self) {
        let target = self.meta.log_target();
        log::debug!(
            target: &target,
            "<id: {}> to {}",
            self.meta.id,
            self.meta.dst
        )
    }

    pub(crate) fn print_publish_err_full(&self)
    where
        E: traits::Event<N>,
    {
        let target = self.meta.log_target();
        log::error!(
            target: &target,
            "Event {:?} with id {} doesn't sent to {}, cause it's full",
            self.data,
            self.meta.id,
            self.meta.dst,
        )
    }
}

impl<N, E> Clone for Event<N, E>
where
    N: crate::traits::Notifier,
    E: traits::Event<N>,
{
    fn clone(&self) -> Self {
        Self {
            meta: self.meta,
            data: self.data.clone(),
            _phantom: Default::default(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Metadata {
    pub(crate) id: usize,
    pub(crate) src: &'static crate::Metadata,
    pub(crate) dst: &'static crate::Metadata,
}

impl core::fmt::Debug for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventMetadata")
            .field("event_id", &self.id)
            .field("src", &format!("{}", self.src))
            .field("dst", &format!("{}", self.dst))
            .finish()
    }
}

impl Metadata {
    pub(crate) fn log_target(&self) -> String {
        format!("Pub<{}>", self.src)
    }

    pub(crate) fn print_publish_err_timeout(&self, timeout: Duration) {
        let target = self.log_target();
        log::error!(
            target: &target,
            "Event<{}> doesn't sent to {}, cause timeout {}",
            self.id,
            self.dst,
            timeout
        )
    }

    pub(crate) fn print_publish_err_inactive(&self) {
        let target = self.log_target();
        log::error!(
            target: &target,
            "Event<{}> doesn't sent to {}, cause it's inactive",
            self.id,
            self.dst,
        )
    }

    // pub(crate) fn print_response_err(&self, id: usize) {
    //     let target = self.log_target();
    //     log::error!(
    //         target: &target,
    //         "Event<{}> doesn't sent to {}, cause it's inactive",
    //         self.id,
    //         self.dst,
    //     )
    // }
}

pub mod traits {
    use crate::{service::traits::Service, traits::Notifier};
    pub trait Event<N: Notifier>: core::fmt::Debug + Clone + 'static {
        type Service: Service<N>;
    }
}
