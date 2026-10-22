//! `serval::time`: waiting, and `Instant` and `Duration` as `rune-modules`'
//! `time` has them (its own are of tokio and `std::time::Instant`, of no
//! clock on the web).

use std::cmp::Ordering;

use rune::{
    Any, ContextError, Module, Value,
    alloc::fmt::TryWrite,
    runtime::{Formatter, Function, Ref, VmResult},
};

use super::host;
use crate::plugin::ErrorText;

pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("serval", ["time"])?;
    module.function("sleep", sleep).build()?;
    module.function("sleep_until", sleep_until).build()?;
    module.ty::<Duration>()?;
    module.function_meta(Duration::new__meta)?;
    module.function_meta(Duration::from_secs__meta)?;
    module.function_meta(Duration::from_millis__meta)?;
    module.function_meta(Duration::from_micros__meta)?;
    module.function_meta(Duration::from_nanos__meta)?;
    module.function_meta(Duration::from_secs_f64__meta)?;
    module.function_meta(Duration::is_zero__meta)?;
    module.function_meta(Duration::as_secs__meta)?;
    module.function_meta(Duration::subsec_millis__meta)?;
    module.function_meta(Duration::subsec_micros__meta)?;
    module.function_meta(Duration::subsec_nanos__meta)?;
    module.function_meta(Duration::as_millis__meta)?;
    module.function_meta(Duration::as_micros__meta)?;
    module.function_meta(Duration::as_nanos__meta)?;
    module.function_meta(Duration::as_secs_f64__meta)?;
    module.function_meta(Duration::add__meta)?;
    module.function_meta(Duration::add_assign__meta)?;
    module.function_meta(Duration::partial_eq__meta)?;
    module.function_meta(Duration::eq__meta)?;
    module.function_meta(Duration::partial_cmp__meta)?;
    module.function_meta(Duration::cmp__meta)?;
    module.function_meta(Duration::debug_fmt__meta)?;
    module.function_meta(Duration::clone__meta)?;

    module.ty::<Instant>()?;
    module.function_meta(Instant::now__meta)?;
    module.function_meta(Instant::duration_since__meta)?;
    module.function_meta(Instant::elapsed__meta)?;
    module.function_meta(Instant::add__meta)?;
    module.function_meta(Instant::add_assign__meta)?;
    module.function_meta(Instant::partial_eq__meta)?;
    module.function_meta(Instant::eq__meta)?;
    module.function_meta(Instant::partial_cmp__meta)?;
    module.function_meta(Instant::cmp__meta)?;
    module.function_meta(Instant::debug_fmt__meta)?;
    module.function_meta(Instant::clone__meta)?;
    Ok(module)
}

/// Waits `duration`, by its ms. Taken by reference: the plugin's own is left to
/// it, to wait again.
async fn sleep(duration: Ref<Duration>) -> Result<(), String> {
    let time = host()?.time;
    let ms = u32::try_from(duration.inner.as_millis()).unwrap_or(u32::MAX);
    let wait = time.after_ms(ms);
    wait.await;
    Ok(())
}

/// How often `sleep_until` asks its condition again.
const CONDITION_CHECK_MS: u32 = 10;

/// Waits until `condition` (a function of no argument, async or not) gives
/// `true`, checked every [`CONDITION_CHECK_MS`]; an `Err` from it is returned.
async fn sleep_until(condition: Function) -> Result<(), String> {
    let time = host()?.time;
    loop {
        let mut met: Value = condition.call(()).into_result().err_text()?;
        if let Ok(future) = met.clone().into_future() {
            met = future.await.into_result().err_text()?;
        }
        let met = match rune::from_value::<bool>(met.clone()) {
            Ok(met) => met,
            Err(_) => match rune::from_value::<Result<bool, Value>>(met.clone()) {
                Ok(Ok(met)) => met,
                Ok(Err(error)) => return Err(format!("The condition failed: {error:?}")),
                Err(_) => {
                    let found = met.type_info();
                    return Err(format!("The condition gave a {found}, not a bool"));
                }
            },
        };
        if met {
            return Ok(());
        }
        time.after_ms(CONDITION_CHECK_MS).await;
    }
}

/// A span of time.
#[derive(Any, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[rune(item = ::serval::time)]
pub struct Duration {
    pub inner: std::time::Duration,
}

/// Rune's integers are `i64`: past it, as many as it holds.
fn saturated(value: u128) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

impl Duration {
    fn of(inner: std::time::Duration) -> Self {
        Self { inner }
    }

    #[rune::function(keep, path = Self::new)]
    fn new(secs: u64, nanos: u32) -> VmResult<Self> {
        if u64::from(nanos / 1_000_000_000) > u64::MAX - secs {
            return VmResult::panic("Duration overflowed");
        }
        VmResult::Ok(Self::of(std::time::Duration::new(secs, nanos)))
    }

    #[rune::function(keep, path = Self::from_secs)]
    fn from_secs(secs: u64) -> Self {
        Self::of(std::time::Duration::from_secs(secs))
    }

    #[rune::function(keep, path = Self::from_millis)]
    pub fn from_millis(millis: u64) -> Self {
        Self::of(std::time::Duration::from_millis(millis))
    }

    #[rune::function(keep, path = Self::from_micros)]
    fn from_micros(micros: u64) -> Self {
        Self::of(std::time::Duration::from_micros(micros))
    }

    #[rune::function(keep, path = Self::from_nanos)]
    fn from_nanos(nanos: u64) -> Self {
        Self::of(std::time::Duration::from_nanos(nanos))
    }

    #[rune::function(keep, path = Self::from_secs_f64)]
    fn from_secs_f64(secs: f64) -> VmResult<Self> {
        match std::time::Duration::try_from_secs_f64(secs) {
            Ok(inner) => VmResult::Ok(Self::of(inner)),
            Err(error) => VmResult::panic(error.to_string()),
        }
    }

    #[rune::function(keep, instance)]
    fn is_zero(&self) -> bool {
        self.inner.is_zero()
    }

    #[rune::function(keep, instance)]
    fn as_secs(&self) -> i64 {
        saturated(u128::from(self.inner.as_secs()))
    }

    #[rune::function(keep, instance)]
    fn subsec_millis(&self) -> u32 {
        self.inner.subsec_millis()
    }

    #[rune::function(keep, instance)]
    fn subsec_micros(&self) -> u32 {
        self.inner.subsec_micros()
    }

    #[rune::function(keep, instance)]
    fn subsec_nanos(&self) -> u32 {
        self.inner.subsec_nanos()
    }

    #[rune::function(keep, instance)]
    fn as_millis(&self) -> i64 {
        saturated(self.inner.as_millis())
    }

    #[rune::function(keep, instance)]
    fn as_micros(&self) -> i64 {
        saturated(self.inner.as_micros())
    }

    #[rune::function(keep, instance)]
    fn as_nanos(&self) -> i64 {
        saturated(self.inner.as_nanos())
    }

    #[rune::function(keep, instance)]
    fn as_secs_f64(&self) -> f64 {
        self.inner.as_secs_f64()
    }

    #[rune::function(keep, instance, protocol = ADD)]
    fn add(&self, rhs: &Duration) -> VmResult<Self> {
        match self.inner.checked_add(rhs.inner) {
            Some(inner) => VmResult::Ok(Self::of(inner)),
            None => VmResult::panic("Duration overflowed"),
        }
    }

    #[rune::function(keep, instance, protocol = ADD_ASSIGN)]
    fn add_assign(&mut self, rhs: &Duration) -> VmResult<()> {
        match self.inner.checked_add(rhs.inner) {
            Some(inner) => {
                self.inner = inner;
                VmResult::Ok(())
            }
            None => VmResult::panic("Duration overflowed"),
        }
    }

    #[rune::function(keep, instance, protocol = PARTIAL_EQ)]
    fn partial_eq(&self, rhs: &Self) -> bool {
        self == rhs
    }

    #[rune::function(keep, instance, protocol = EQ)]
    fn eq(&self, rhs: &Self) -> bool {
        self == rhs
    }

    #[rune::function(keep, instance, protocol = PARTIAL_CMP)]
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(self, rhs)
    }

    #[rune::function(keep, instance, protocol = CMP)]
    fn cmp(&self, rhs: &Self) -> Ordering {
        Ord::cmp(self, rhs)
    }

    #[rune::function(keep, instance, protocol = DEBUG_FMT)]
    fn debug_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{:?}", self.inner)
    }

    #[rune::function(keep, instance, protocol = CLONE)]
    fn clone(&self) -> Self {
        *self
    }
}

/// A point in time: Serval's clock (the wall clock), in ms.
#[derive(Any, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[rune(item = ::serval::time)]
pub struct Instant {
    /// Since the Unix epoch
    pub ms: i64,
}

impl Instant {
    #[rune::function(keep, path = Self::now)]
    fn now() -> VmResult<Instant> {
        match host() {
            Ok(host) => VmResult::Ok(Self {
                ms: host.time.current(),
            }),
            Err(error) => VmResult::panic(error),
        }
    }

    /// How long after `earlier` it is: none if it is before it.
    #[rune::function(keep, instance)]
    fn duration_since(&self, earlier: &Instant) -> Duration {
        Duration::from_millis(self.ms.saturating_sub(earlier.ms).max(0) as u64)
    }

    #[rune::function(keep, instance)]
    fn elapsed(&self) -> VmResult<Duration> {
        let now = rune::vm_try!(Self::now());
        VmResult::Ok(now.duration_since(self))
    }

    /// Of the ms of `rhs`: Serval's clock has no finer.
    #[rune::function(keep, instance, protocol = ADD)]
    fn add(&self, rhs: &Duration) -> VmResult<Self> {
        match i64::try_from(rhs.inner.as_millis())
            .ok()
            .and_then(|ms| self.ms.checked_add(ms))
        {
            Some(ms) => VmResult::Ok(Self { ms }),
            None => VmResult::panic("Instant overflowed"),
        }
    }

    #[rune::function(keep, instance, protocol = ADD_ASSIGN)]
    fn add_assign(&mut self, rhs: &Duration) -> VmResult<()> {
        *self = rune::vm_try!(self.add(rhs));
        VmResult::Ok(())
    }

    #[rune::function(keep, instance, protocol = PARTIAL_EQ)]
    fn partial_eq(&self, rhs: &Self) -> bool {
        self == rhs
    }

    #[rune::function(keep, instance, protocol = EQ)]
    fn eq(&self, rhs: &Self) -> bool {
        self == rhs
    }

    #[rune::function(keep, instance, protocol = PARTIAL_CMP)]
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(self, rhs)
    }

    #[rune::function(keep, instance, protocol = CMP)]
    fn cmp(&self, rhs: &Self) -> Ordering {
        Ord::cmp(self, rhs)
    }

    /// As the console shows times.
    #[rune::function(keep, instance, protocol = DEBUG_FMT)]
    fn debug_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        match host() {
            Ok(host) => rune::vm_write!(f, "Instant({})", host.time.format_millis(self.ms)),
            Err(_) => rune::vm_write!(f, "Instant({} ms)", self.ms),
        }
    }

    #[rune::function(keep, instance, protocol = CLONE)]
    fn clone(&self) -> Self {
        *self
    }
}
