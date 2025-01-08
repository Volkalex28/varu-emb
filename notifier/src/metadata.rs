#[allow(dead_code)]
#[derive(Clone)]
pub struct Metadata {
    pub(crate) id: usize,
    pub(crate) index: Option<usize>,
    pub(crate) name: &'static str,
}

pub const fn check<N, S>() -> bool
where
    N: crate::traits::NotifierService<S>,
    S: crate::service::traits::Service<N>,
{
    S::COUNT != 0 && N::ID + S::COUNT <= N::COUNT
}

impl Metadata {
    pub(crate) const fn new<N, S>(index: usize) -> Self
    where
        N: crate::traits::NotifierService<S>,
        S: crate::service::traits::Service<N>,
        varuemb_utils::assert::Assert<{ check::<N, S>() }>: varuemb_utils::assert::IsTrue,
    {
        Self { id: N::ID, index: if S::COUNT == 1 { None } else { Some(index) }, name: N::NAME }
    }

    pub(crate) const fn new_service<N, S>() -> Self
    where
        N: crate::traits::NotifierService<S>,
        S: crate::service::traits::Service<N>,
        varuemb_utils::assert::Assert<{ check::<N, S>() }>: varuemb_utils::assert::IsTrue,
    {
        Self { id: N::ID, index: None, name: N::NAME }
    }

    pub fn is_same(&self, other: &Self) -> bool {
        self.id == other.id && self.index == other.index
    }

    pub fn is_service(&self, other: &Self) -> bool {
        other == self
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl PartialEq for Metadata {
    fn eq(&self, other: &Self) -> bool {
        let id = self.id == other.id;
        let index = match (self.index, other.index) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(this), Some(other)) => this == other,
        };
        id && index
    }
}

impl core::fmt::Display for Metadata {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.index.as_ref() {
            None => write!(f, "{}", self.name),
            Some(index) => write!(f, "{}[{index}]", self.name),
        }
    }
}

impl core::fmt::Debug for Metadata {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self, f)
    }
}
