//! `serval::rand`: pseudo random numbers, as the `rand` crate gives them
//! (`random`, `random_range`, `random_bool`, and an `Rng` of its own seed).
//! Seeded by `getrandom`: the OS's, or the browser's on the web.

use std::cell::RefCell;

use rand::{RngExt, SeedableRng, rngs::SmallRng};
use rune::{Any, ContextError, Module, Value, runtime::VmResult};

/// Pseudo random numbers, as Rust's `rand` crate gives them.
///
/// The functions share a generator seeded by the system (the browser, on the
/// web). For the same numbers each run, make an `Rng` of a seed.
///
/// # Examples
///
/// ```rune
/// use serval::{data, rand, time};
///
/// /// Pushes a noisy sine wave to `noisy`, for trying maps and graphs.
/// pub async fn noise() {
///     let start = time::Instant::now();
///     loop {
///         let t = start.elapsed().as_secs_f64();
///         data::push("noisy", t.sin() + rand::random_range(-0.1, 0.1))?;
///         time::sleep(time::Duration::from_millis(50)).await?;
///     }
/// }
/// ```
#[rune::module(::serval::rand)]
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
    module.function_meta(random__meta)?;
    module.function_meta(random_range__meta)?;
    module.function_meta(random_bool__meta)?;
    module.ty::<Rng>()?;
    module.function_meta(Rng::new__meta)?;
    module.function_meta(Rng::seed_from_u64__meta)?;
    module.function_meta(Rng::random__meta)?;
    module.function_meta(Rng::random_range__meta)?;
    module.function_meta(Rng::random_bool__meta)?;
    Ok(module)
}

thread_local! {
    /// The free functions', seeded as first used
    static RNG: RefCell<Option<SmallRng>> = const { RefCell::new(None) };
}

fn seeded() -> VmResult<SmallRng> {
    match getrandom::u64() {
        Ok(seed) => VmResult::Ok(SmallRng::seed_from_u64(seed)),
        Err(error) => VmResult::panic(format!("No seed for random numbers: {error}")),
    }
}

/// `f` on the free functions' generator.
fn with_rng<T>(f: impl FnOnce(&mut SmallRng) -> VmResult<T>) -> VmResult<T> {
    RNG.with(|cell| {
        let mut rng = cell.borrow_mut();
        if rng.is_none() {
            *rng = Some(rune::vm_try!(seeded()));
        }
        f(rng.as_mut().expect("seeded"))
    })
}

/// A float from `0.0` up to (not including) `1.0`.
///
/// # Examples
///
/// ```rune
/// use serval::rand;
///
/// pub async fn roll() {
///     if rand::random() < 0.5 {
///         println!("heads");
///     }
/// }
/// ```
#[rune::function(keep)]
fn random() -> VmResult<f64> {
    with_rng(|rng| VmResult::Ok(rng.random()))
}

/// A number from `low` up to (not including) `high`.
///
/// An integer of two integers, a float of two floats.
///
/// # Examples
///
/// ```rune
/// use serval::rand;
///
/// pub async fn dice() {
///     let die = rand::random_range(1, 7);
///     let jitter = rand::random_range(-0.5, 0.5);
///     println!("{} {}", die, jitter);
/// }
/// ```
#[rune::function(keep)]
fn random_range(low: Value, high: Value) -> VmResult<Value> {
    with_rng(|rng| in_range(rng, low, high))
}

/// `true` with the probability `p`, from `0.0` to `1.0`.
///
/// # Examples
///
/// ```rune
/// use serval::{rand, serial};
///
/// /// Sends a corrupted frame one time in ten, to test the device.
/// pub async fn faulty() {
///     if rand::random_bool(0.1) {
///         serial::send([0x02, 0xFF, 0x03])?;
///     } else {
///         serial::send([0x02, 0x41, 0x03])?;
///     }
/// }
/// ```
#[rune::function(keep)]
fn random_bool(p: f64) -> VmResult<bool> {
    with_rng(|rng| bool_of(rng, p))
}

fn in_range(rng: &mut SmallRng, low: Value, high: Value) -> VmResult<Value> {
    if let (Ok(low), Ok(high)) = (low.as_integer::<i64>(), high.as_integer::<i64>()) {
        if low >= high {
            return VmResult::panic(format!("Empty range: {low}..{high}"));
        }
        return VmResult::Ok(Value::from(rng.random_range(low..high)));
    }
    match (
        rune::from_value::<f64>(low.clone()),
        rune::from_value::<f64>(high.clone()),
    ) {
        (Ok(low), Ok(high)) if low < high && (high - low).is_finite() => {
            VmResult::Ok(Value::from(rng.random_range(low..high)))
        }
        (Ok(low), Ok(high)) => VmResult::panic(format!("Not a range to take from: {low}..{high}")),
        _ => VmResult::panic(format!(
            "A range of two integers or two floats, not of {} and {}",
            low.type_info(),
            high.type_info()
        )),
    }
}

fn bool_of(rng: &mut SmallRng, p: f64) -> VmResult<bool> {
    if (0.0..=1.0).contains(&p) {
        VmResult::Ok(rng.random_bool(p))
    } else {
        VmResult::panic(format!("Not a probability: {p}"))
    }
}

/// A random number generator of its own, made of a seed.
///
/// Of the same seed, it gives the same numbers each run.
///
/// It has the methods `random`, `random_range`, and `random_bool`, as the
/// module's functions.
///
/// # Examples
///
/// ```rune
/// use serval::rand;
///
/// pub async fn repeatable() {
///     let rng = rand::Rng::seed_from_u64(42);
///     println!("{} {}", rng.random(), rng.random_range(0, 10));
/// }
/// ```
#[derive(Any, Debug)]
#[rune(item = ::serval::rand)]
pub struct Rng {
    inner: SmallRng,
}

impl Rng {
    /// A generator seeded by the system.
    #[rune::function(keep, path = Self::new)]
    fn new() -> VmResult<Self> {
        VmResult::Ok(Self {
            inner: rune::vm_try!(seeded()),
        })
    }

    /// A generator of `seed`: the same seed gives the same numbers.
    #[rune::function(keep, path = Self::seed_from_u64)]
    fn seed_from_u64(seed: i64) -> Self {
        Self {
            inner: SmallRng::seed_from_u64(seed as u64),
        }
    }

    #[rune::function(keep, instance)]
    fn random(&mut self) -> f64 {
        self.inner.random()
    }

    #[rune::function(keep, instance)]
    fn random_range(&mut self, low: Value, high: Value) -> VmResult<Value> {
        in_range(&mut self.inner, low, high)
    }

    #[rune::function(keep, instance)]
    fn random_bool(&mut self, p: f64) -> VmResult<bool> {
        bool_of(&mut self.inner, p)
    }
}

#[cfg(test)]
mod tests {
    use crate::plugin::testing::run_body;

    fn run(body: &str) -> Result<Option<String>, String> {
        run_body(&format!("use serval::rand;\n{body}"))
    }

    #[test]
    fn random_numbers_are_in_their_ranges() {
        let body = "let ok = true;\n\
            for _ in 0..1000 {\n\
                let x = rand::random();\n\
                let i = rand::random_range(-3, 3);\n\
                let f = rand::random_range(1.5, 2.0);\n\
                ok = ok && x >= 0.0 && x < 1.0 && i >= -3 && i < 3 && f >= 1.5 && f < 2.0;\n\
            }\n\
            ok && !rand::random_bool(0.0) && rand::random_bool(1.0)";
        assert_eq!(run(body), Ok(Some("true".into())));
    }

    #[test]
    fn a_seed_gives_the_same_numbers() {
        let body = "let a = rand::Rng::seed_from_u64(42);\n\
            let b = rand::Rng::seed_from_u64(42);\n\
            let c = rand::Rng::new();\n\
            let same = a.random() == b.random() && a.random_range(0, 1000) == b.random_range(0, 1000);\n\
            let x = c.random();\n\
            same && x >= 0.0 && x < 1.0";
        assert_eq!(run(body), Ok(Some("true".into())));
    }

    #[test]
    fn what_is_no_range_is_told() {
        assert!(
            run("rand::random_range(3, 3)")
                .unwrap_err()
                .contains("Empty range")
        );
        assert!(
            run("rand::random_range(1, 2.0)")
                .unwrap_err()
                .contains("two integers or two floats")
        );
        assert!(
            run("rand::random_bool(1.5)")
                .unwrap_err()
                .contains("Not a probability")
        );
    }
}
