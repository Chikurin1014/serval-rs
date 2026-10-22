//! The functions of `f64` of `num_traits`' `Float` that Rune's own lacks, as
//! methods (`x.sin()`, `f64::nan()`), and the constants of its `FloatConst`
//! (`f64::PI()`).

use rune::{ContextError, Module, runtime::VmResult};

/// Each as a method of `f64`, of the same name as Rust's.
macro_rules! methods {
    ($module:ident, $($name:ident),* $(,)?) => {
        $(
            $module
                .function(stringify!($name), |x: f64| x.$name())
                .build_associated::<f64>()?;
        )*
    };
}

/// Each as a function of `f64` of no argument, as `FloatConst`'s.
macro_rules! constants {
    ($module:ident, $($name:ident),* $(,)?) => {
        $(
            $module
                .function(stringify!($name), || std::f64::consts::$name)
                .build_associated::<f64>()?;
        )*
    };
}

/// Each as a function of `f64` of no argument giving its value, as `Float`'s.
macro_rules! associated {
    ($module:ident, $($name:ident => $value:expr),* $(,)?) => {
        $(
            $module
                .function(stringify!($name), || -> f64 { $value })
                .build_associated::<f64>()?;
        )*
    };
}

pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("serval", ["math"])?;
    methods!(
        module, sin, cos, tan, asin, acos, atan, sinh, cosh, tanh, asinh, acosh, atanh, exp, exp2,
        exp_m1, ln, ln_1p, log2, log10, to_degrees, to_radians,
    );
    module
        .function("atan2", |y: f64, x: f64| y.atan2(x))
        .build_associated::<f64>()?;
    module
        .function("sin_cos", |x: f64| x.sin_cos())
        .build_associated::<f64>()?;
    module
        .function("log", |x: f64, base: f64| x.log(base))
        .build_associated::<f64>()?;
    constants!(
        module,
        E,
        FRAC_1_PI,
        FRAC_1_SQRT_2,
        FRAC_2_PI,
        FRAC_2_SQRT_PI,
        FRAC_PI_2,
        FRAC_PI_3,
        FRAC_PI_4,
        FRAC_PI_6,
        FRAC_PI_8,
        LN_10,
        LN_2,
        LOG10_E,
        LOG10_2,
        LOG2_E,
        LOG2_10,
        PI,
        SQRT_2,
        TAU,
    );
    methods!(
        module,
        trunc,
        fract,
        signum,
        recip,
        cbrt,
        is_sign_positive,
        is_sign_negative,
    );
    module
        .function("hypot", |x: f64, other: f64| x.hypot(other))
        .build_associated::<f64>()?;
    module
        .function("copysign", |x: f64, sign: f64| x.copysign(sign))
        .build_associated::<f64>()?;
    module
        .function("mul_add", |x: f64, a: f64, b: f64| x.mul_add(a, b))
        .build_associated::<f64>()?;
    // As `num_traits` has it: `std` deprecates its own
    module
        .function("abs_sub", |x: f64, other: f64| (x - other).max(0.0))
        .build_associated::<f64>()?;
    module
        .function("clamp", |x: f64, min: f64, max: f64| {
            if min <= max {
                VmResult::Ok(x.clamp(min, max))
            } else {
                VmResult::panic(format!("Cannot clamp to {min}..={max}"))
            }
        })
        .build_associated::<f64>()?;
    module
        .function("integer_decode", integer_decode)
        .build_associated::<f64>()?;
    associated!(
        module,
        nan => f64::NAN,
        infinity => f64::INFINITY,
        neg_infinity => f64::NEG_INFINITY,
        neg_zero => -0.0,
        min_value => f64::MIN,
        min_positive_value => f64::MIN_POSITIVE,
        max_value => f64::MAX,
        epsilon => f64::EPSILON,
    );
    Ok(module)
}

/// Its mantissa, exponent and sign, as `num_traits` decodes it: `x` is
/// `sign * mantissa * 2^exponent`.
fn integer_decode(x: f64) -> (i64, i64, i64) {
    let bits = x.to_bits();
    let sign = if bits >> 63 == 0 { 1 } else { -1 };
    let mut exponent = ((bits >> 52) & 0x7ff) as i64;
    let mantissa = if exponent == 0 {
        (bits & 0xf_ffff_ffff_ffff) << 1
    } else {
        (bits & 0xf_ffff_ffff_ffff) | 0x10_0000_0000_0000
    };
    exponent -= 1023 + 52;
    // Of 53 bits at most: an `i64` holds it
    (mantissa as i64, exponent, sign)
}

#[cfg(test)]
mod tests {
    use crate::plugin::testing::run_body as run;

    #[test]
    fn floats_have_the_functions_of_num_traits() {
        assert_eq!(
            run("f64::PI().sin().abs() < 1e-15"),
            Ok(Some("true".into()))
        );
        assert_eq!(run("0.0.cos()"), Ok(Some("1.0".into())));
        assert_eq!(
            run("1.0.atan2(1.0) == f64::FRAC_PI_4()"),
            Ok(Some("true".into()))
        );
        assert_eq!(run("0.0.sin_cos()"), Ok(Some("(0.0, 1.0)".into())));
        assert_eq!(run("1.0.exp() == f64::E()"), Ok(Some("true".into())));
        assert_eq!(run("f64::E().ln()"), Ok(Some("1.0".into())));
        assert_eq!(run("8.0.log2()"), Ok(Some("3.0".into())));
        assert_eq!(run("1000.0.log10()"), Ok(Some("3.0".into())));
        assert_eq!(run("9.0.log(3.0)"), Ok(Some("2.0".into())));
        assert_eq!(
            run("180.0.to_radians() == f64::PI()"),
            Ok(Some("true".into()))
        );
        assert_eq!(run("0.0.tanh()"), Ok(Some("0.0".into())));
        assert_eq!(
            run("f64::TAU() / 2.0 == f64::PI()"),
            Ok(Some("true".into()))
        );
    }

    #[test]
    fn floats_have_the_rest_of_float() {
        assert_eq!(run("(-2.7).trunc()"), Ok(Some("-2.0".into())));
        assert_eq!(run("2.5.fract()"), Ok(Some("0.5".into())));
        assert_eq!(run("(-3.0).signum()"), Ok(Some("-1.0".into())));
        assert_eq!(run("4.0.recip()"), Ok(Some("0.25".into())));
        assert_eq!(run("27.0.cbrt()"), Ok(Some("3.0".into())));
        assert_eq!(run("3.0.hypot(4.0)"), Ok(Some("5.0".into())));
        assert_eq!(run("3.0.copysign(-1.0)"), Ok(Some("-3.0".into())));
        assert_eq!(run("2.0.mul_add(3.0, 1.0)"), Ok(Some("7.0".into())));
        assert_eq!(run("1.0.abs_sub(3.0)"), Ok(Some("0.0".into())));
        assert_eq!(run("5.0.clamp(0.0, 1.0)"), Ok(Some("1.0".into())));
        assert!(
            run("5.0.clamp(1.0, 0.0)")
                .unwrap_err()
                .contains("Cannot clamp")
        );
        assert_eq!(
            run("f64::neg_zero().is_sign_negative()"),
            Ok(Some("true".into()))
        );
        assert_eq!(run("f64::nan().is_nan()"), Ok(Some("true".into())));
        assert_eq!(run("f64::max_value() == f64::MAX"), Ok(Some("true".into())));
        assert_eq!(
            run("f64::epsilon() == f64::EPSILON"),
            Ok(Some("true".into()))
        );
        // 2.0 is 2^52 * 2^-51
        assert_eq!(
            run("2.0.integer_decode()"),
            Ok(Some("(4503599627370496, -51, 1)".into()))
        );
    }
}
