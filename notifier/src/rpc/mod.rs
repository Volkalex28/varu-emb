use crate::event::{self, traits as __evt};
use crate::pubsub::{self, traits as __pubsub};
use crate::service::traits as __svc;
use crate::traits::*;
use core::ops::{Deref, Index};
use embassy_time::{Duration, Timer};
use futures_util::future::pending;
use futures_util::FutureExt;
use varuemb_utils::assert::*;
use varuemb_utils::select;

pub mod traits;

pub type Result<T, R: traits::Rpc> =
    core::result::Result<T, pubsub::Error<R::Notifier, Request<R>, GetResponseError<R, R::Service>>>;
pub type GetResponse<P, R> = Response<pubsub::GetPubSub<P, R>>;
pub type GetRequest<P, R> = Request<pubsub::GetPubSub<P, R>>;
pub type GetRpcRequest<S, N> = RpcRequest<<S as __svc::Service<N>>::Impl>;
pub type GetRpcSubscriber<S, N> = pubsub::Subscriber<N, Request<crate::GetPubSub<N, S>>>;
pub type GetRpcRequestError<S, N> = pubsub::Error<N, Request<crate::GetPubSub<N, S>>, <S as traits::RpcProvider<N>>::Error>;

pub struct Container<R: traits::Rpc, const C: usize>
where
    R::Service: traits::RpcProvider<R::Notifier>,
{
    inner: [traits::GetRpc<R>; C],
}

impl<R: traits::Rpc, const C: usize> Container<R, C>
where
    R::Service: traits::RpcProvider<R::Notifier>,
{
    pub(crate) fn new(
        channel: __pubsub::GetSubscriberRet<R::Notifier, Response<R>>,
        meta: &'static crate::Metadata,
    ) -> Self {
        Self { inner: core::array::from_fn(|index| R::__new_rpc(Rpc { index, src: meta, channel })) }
    }
}

impl<R: traits::Rpc, const C: usize> Deref for Container<R, C>
where
    Assert<{ C == 1 }>: IsTrue,
    R::Service: traits::RpcProvider<R::Notifier>,
{
    type Target = traits::GetRpc<R>;
    fn deref(&self) -> &Self::Target {
        &self.inner[0]
    }
}

impl<I, R: traits::Rpc, const C: usize> Index<I> for Container<R, C>
where
    Assert<{ C > 1 }>: IsTrue,
    R::Service: traits::RpcProvider<R::Notifier>,
    I: core::slice::SliceIndex<[traits::GetRpc<R>]>,
{
    type Output = I::Output;
    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

pub struct Subscription<P, S, const C: usize>
where
    P: __pubsub::PubSub,
    S: __svc::Service<P::Notifier, Impl: traits::Rpc> + traits::RpcProvider<P::Notifier>,
{
    pubsub: pubsub::Subscription<P, Response<S::Impl>, C>,
}
impl<P, S, const C: usize> Subscription<P, S, C>
where
    P: __pubsub::PubSub,
    Response<S::Impl>: __pubsub::IsPublisher<P>,
    Msg<{ pubsub::assert::subscriber::<P, Response<S::Impl>>() }>: IsTrue,
    S: __svc::Service<P::Notifier, Impl: traits::Rpc> + traits::RpcProvider<P::Notifier>,
{
    pub const fn default() -> Self {
        Self { pubsub: pubsub::Subscription::default() }
    }
}
impl<P, S, const C: usize> Deref for Subscription<P, S, C>
where
    P: __pubsub::PubSub,
    S: __svc::Service<P::Notifier, Impl: traits::Rpc> + traits::RpcProvider<P::Notifier>,
{
    type Target = pubsub::Subscription<P, Response<S::Impl>, C>;
    fn deref(&self) -> &Self::Target {
        &self.pubsub
    }
}

pub struct Request<R: traits::Rpc>
where
    R::Service: traits::RpcProvider<R::Notifier>,
{
    src: &'static crate::Metadata,
    data: <R::Service as traits::RpcProvider<R::Notifier>>::Request,
}
impl<R: traits::Rpc> core::fmt::Debug for Request<R>
where
    R::Service: traits::RpcProvider<R::Notifier>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Request")
            .field("src", &format_args!("{}", self.src))
            .field("data", &self.data)
            .finish()
    }
}
impl<R: traits::Rpc> Clone for Request<R>
where
    R::Service: traits::RpcProvider<R::Notifier>,
{
    fn clone(&self) -> Self {
        Self { src: self.src, data: self.data.clone() }
    }
}
impl<R: traits::Rpc> __evt::Event<R::Notifier> for Request<R>
where
    R::Service: traits::RpcProvider<R::Notifier>,
{
    type Service = R::Service;
}

type GetResponseData<R: traits::Rpc<Service = S>, S: traits::RpcProvider<R::Notifier>> = S::Response;
type GetResponseError<R: traits::Rpc<Service = S>, S: traits::RpcProvider<R::Notifier>> = S::Error;
type GetResponseRes<R: traits::Rpc<Service = S>, S: traits::RpcProvider<R::Notifier>> =
    core::result::Result<GetResponseData<R, S>, GetResponseError<R, S>>;

pub struct Response<R: traits::Rpc>
where
    R::Service: traits::RpcProvider<R::Notifier>,
{
    pub id: usize,
    pub data: GetResponseRes<R, R::Service>,
}
impl<R: traits::Rpc> core::fmt::Debug for Response<R>
where
    R::Service: traits::RpcProvider<R::Notifier>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Response").field("id", &self.id).field("data", &self.data).finish()
    }
}
impl<R: traits::Rpc> Clone for Response<R>
where
    R::Service: traits::RpcProvider<R::Notifier>,
{
    fn clone(&self) -> Self {
        Self { id: self.id, data: self.data.clone() }
    }
}
impl<R: traits::Rpc> __evt::Event<R::Notifier> for Response<R>
where
    R::Service: traits::RpcProvider<R::Notifier>,
{
    type Service = R::Service;
}

pub struct RpcRequest<R: traits::Rpc>
where
    R::Service: traits::RpcProvider<R::Notifier>,
{
    id: usize,
    req_discriminant: usize,
    src: &'static crate::Metadata,
    pubsub: &'static pubsub::PubSub<R>,
    data: Option<<R::Service as traits::RpcProvider<R::Notifier>>::Request>,
}
impl<Req, Resp, RespE, R: traits::Rpc> RpcRequest<R>
where
    Req: __evt::Event<R::Notifier>,
    Resp: __evt::Event<R::Notifier>,
    RespE: core::fmt::Debug + Clone + 'static,
    R::Service: traits::RpcProvider<R::Notifier, Request = Req, Response = Resp, Error = RespE>,
{
    pub fn take(&mut self) -> Req {
        self.data.take().expect("Data already taken")
    }

    pub fn peak(&self) -> &Req {
        self.data.as_ref().expect("Data already taken")
    }

    pub fn src(&self) -> &'static crate::Metadata {
        self.src
    }

    pub async fn response(&self, resp: Resp) -> Result<(), R>
    where
        R::Notifier: NotifierService<R::Service>,
        pubsub::PubSub<R>: __pubsub::CanPublish<Response<R>, Notifier = R::Notifier>,
        for<'r> &'r Resp: Into<usize>,
    {
        self.raw_response(resp, None).await
    }

    pub async fn response_with(&self, resp: Resp, timeout: crate::Duration) -> Result<(), R>
    where
        R::Notifier: NotifierService<R::Service>,
        pubsub::PubSub<R>: __pubsub::CanPublish<Response<R>, Notifier = R::Notifier>,
        for<'r> &'r Resp: Into<usize>,
    {
        self.raw_response(resp, Some(timeout)).await
    }

    async fn raw_response(&self, resp: Resp, timeout: Option<crate::Duration>) -> Result<(), R>
    where
        R::Notifier: NotifierService<R::Service>,
        pubsub::PubSub<R>: __pubsub::CanPublish<Response<R>, Notifier = R::Notifier>,
        for<'r> &'r Resp: Into<usize>,
    {
        let resp_discriminant: usize = (&resp).into();
        if self.req_discriminant != resp_discriminant {
            return Err(pubsub::Error::IncorrectResponse(self.src, self.id));
        }
        self.pubsub
            .publisher()
            .set_targets([self.src])
            .allow_inactive(false)
            .inactive_is_err(true)
            .publish_with(Response { id: self.id, data: Ok(resp) }, timeout)
            .await;
        Ok(())
    }

    pub fn response_err(&self, err: RespE) -> Result<(), R>
    where
        R::Notifier: NotifierService<R::Service>,
        pubsub::PubSub<R>: __pubsub::CanPublish<Response<R>, Notifier = R::Notifier>,
    {
        self.pubsub
            .publisher()
            .set_targets([self.src])
            .allow_inactive(false)
            .inactive_is_err(true)
            .publish(Response { id: self.id, data: Err(err) });
        Ok(())
    }
}

pub struct Rpc<R: traits::Rpc>
where
    R::Service: traits::RpcProvider<R::Notifier>,
{
    index: usize,
    src: &'static crate::Metadata,
    channel: __pubsub::GetSubscriberRet<R::Notifier, Response<R>>,
}

impl<Req, Resp, N, R, S> Rpc<R>
where
    N: Notifier,
    Req: __evt::Event<N>,
    Resp: __evt::Event<N>,
    R: traits::Rpc<Notifier = N, Service = S> + __pubsub::CanMetadata,
    S: traits::RpcProvider<N, Request = Req, Response = Resp, Impl = R> + 'static,
{
    fn publisher(index: usize) -> &'static pubsub::PubSub<R>
    where
        [(); S::COUNT]:,
        N: NotifierService<S>,
    {
        let c = S::notif();
        __pubsub::GetPubSub::<R>::__get(c, index)
    }

    fn subscriber(&self) -> pubsub::Subscriber<R::Notifier, Response<R>> {
        pubsub::Subscriber::new(self.channel)
    }

    pub fn process_send_only(&self, req: Req) -> Result<(), R>
    where
        [(); S::COUNT]:,
        N: NotifierService<S>,
        pubsub::PubSub<R>: __pubsub::CanPublish<Request<R>, Notifier = N>,
    {
        let publisher = Self::publisher(self.index);

        let mut err = None;
        publisher
            .publisher()
            .break_after_error(true)
            .set_targets([publisher.metadata()])
            .set_error_handler::<_, GetResponseError<R, S>>(|e| err = Some(e))
            .publish(Request { src: self.src, data: req });
        if let Some(err) = err {
            use __pubsub::CanPublish;
            pubsub::PubSub::<R>::print_error(&err);
            return Err(err);
        }
        Ok(())
    }

    pub async fn process<D>(
        &self,
        req: Req,
        timeout: Option<Duration>,
        cb: core::result::Result<impl Fn(Resp) -> Option<D>, D>,
    ) -> Result<D, R>
    where
        [(); S::COUNT]:,
        N: NotifierService<S>,
        pubsub::PubSub<R>: __pubsub::CanPublish<Request<R>, Notifier = N>,
    {
        let publisher = Self::publisher(self.index);
        let mut subscriber = self.subscriber();

        let mut err = None;
        let res = publisher
            .publisher()
            .break_after_error(true)
            .set_targets([publisher.metadata()])
            .set_error_handler::<_, GetResponseError<R, S>>(|e| err = Some(e))
            .publish_with(Request { src: self.src, data: req }, timeout)
            .await;
        if let Some(err) = err {
            use __pubsub::CanPublish;
            pubsub::PubSub::<R>::print_error(&err);
            return Err(err);
        }

        let cb = match cb {
            Ok(cb) => cb,
            Err(ret) => return Ok(ret),
        };

        let meta = publisher.metadata();
        let response = async move {
            loop {
                let event = subscriber.next().await;
                if event.meta.src != publisher.metadata() {
                    continue;
                }

                let resp = event.data();
                if resp.id != res.id {
                    continue;
                }
                break match resp.data {
                    Ok(resp) => match (cb)(resp) {
                        Some(data) => Ok(data),
                        None => Err(pubsub::Error::IncorrectResponse(self.src, res.id)),
                    },
                    Err(err) => Err(pubsub::Error::Response(self.src, res.id, err)),
                };
            }
        };
        let timeout = async {
            match timeout {
                Some(duration) => Timer::after(duration).map(|_| duration).await,
                None => pending().await,
            }
        };
        select! {
            response = response => { response }
            duration = timeout => { Err(pubsub::Error::Timeout(event::Metadata { id: res.id, src: self.src, dst: meta }, duration)) }
        }
    }

    pub(crate) async fn request<'a>(subscriber: &mut pubsub::Subscriber<N, Request<R>>) -> RpcRequest<R>
    where
        [(); S::COUNT]:,
        N: NotifierService<S>,
        for<'r> &'r Req: Into<usize>,
    {
        let event::Event { data: Request { src, data }, meta: event::Metadata { id, dst, .. }, .. } =
            subscriber.next().await;

        let pubsub = Self::publisher(dst.index.unwrap_or_default());

        RpcRequest { id, src, pubsub, req_discriminant: (&data).into(), data: Some(data) }
    }
}
