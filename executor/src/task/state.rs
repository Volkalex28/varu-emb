use core::{
    fmt,
    sync::atomic::{AtomicU32, Ordering::*},
};

#[allow(unused)]
#[repr(u32)]
enum Bits {
    Spawned = 0,
    Finished = 1,
    Ready = 2,
    Running = 3,
}

const SPAWNED: u32 = 1 << Bits::Spawned as u32;
const FINISHED: u32 = 1 << Bits::Finished as u32;
#[allow(unused)]
const READY: u32 = 1 << Bits::Ready as u32;
const RUNNING: u32 = 1 << Bits::Running as u32;

proc_bitfield::bitfield! {
    struct Repr(u32): Debug {
        spawned: bool @ 0,
        finished: bool @ 1,
        ready: bool @ 2,
        running: bool @ 3,
    }
}
pub struct State(AtomicU32);
impl State {
    pub const fn new() -> Self {
        Self(AtomicU32::new(0))
    }

    #[inline]
    pub fn spawn(&self) -> bool {
        self.0.compare_exchange(0, SPAWNED, SeqCst, SeqCst).is_ok()
    }

    #[inline]
    pub fn despawn(&self) {
        self.0.store(0, SeqCst);
    }

    #[inline]
    pub fn finish(&self) -> bool {
        self.0.fetch_or(FINISHED, SeqCst) & READY != 0
    }

    #[inline]
    pub fn ready(&self) -> bool {
        self.update(|this| (this.spawned() && !this.ready() && !this.finished()).then(|| this.with_ready(true))).is_ok()
    }

    // #[inline]
    // pub fn is_ready(&self) -> bool {
    //     self.0.load(SeqCst) & READY != 0
    // }

    #[inline]
    pub fn begin(&self) -> bool {
        !self.update(|this| Some(this.with_ready(false).with_running(true))).unwrap().finished()
    }

    #[inline]
    pub fn end(&self) {
        let this = Repr(self.0.fetch_and(!RUNNING, SeqCst));
        if this.finished() && !this.ready() {
            self.despawn();
        }
    }

    fn update(&self, mut f: impl FnMut(Repr) -> Option<Repr>) -> Result<Repr, Repr> {
        let mut current = self.0.load(SeqCst);
        while let Some(new) = f(Repr(current)) {
            match self.0.compare_exchange(current, new.0, SeqCst, SeqCst) {
                Ok(value) => return Ok(Repr(value)),
                Err(next) => current = next,
            }
        }
        Err(Repr(current))
    }
}
impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("State").field(&Repr(self.0.load(Relaxed))).finish()
    }
}
impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let this = Repr(self.0.load(Relaxed));

        f.debug_tuple("State")
            .field_with(|f| {
                if !this.spawned() {}

                if !this.ready() && !this.finished() && !this.running() {
                    return f.write_str("Blocked");
                }

                let mut first = true;
                let mut writer = |bit: &str| -> fmt::Result {
                    if !first {
                        f.write_str(" | ")?;
                    }
                    first = false;
                    f.write_str(bit)?;
                    Ok(())
                };

                if this.ready() {
                    writer("Ready")?;
                }
                if this.finished() {
                    writer("Finished")?;
                }
                if this.running() {
                    writer("Running")?;
                }

                Ok(())
            })
            .finish()
    }
}
