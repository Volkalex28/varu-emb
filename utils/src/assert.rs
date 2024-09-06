pub use const_assert::*;

pub struct Msg<const MSG: &'static str> {}
impl IsTrue for Msg<""> {}

#[rustfmt::skip]
    pub const fn assert(cond: bool, msg: &'static str) -> &'static str {
        if cond { "" } else if msg.is_empty() { "Bad condition" } else { msg }
    }

pub struct AssertMsg<const COND: bool, const MSG: &'static str = ""> {}
impl<const COND: bool, const MSG: &'static str> IsTrue for AssertMsg<COND, MSG> where
    Msg<{ assert(COND, MSG) }>: IsTrue
{
}
impl<const COND: bool, const MSG: &'static str> IsFalse for AssertMsg<COND, MSG> where
    Msg<{ assert(!COND, MSG) }>: IsTrue
{
}
