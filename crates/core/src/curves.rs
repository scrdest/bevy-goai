//! This module defines stuff to do with Utility Curves.
//! 
//! There is nothing particularly special or complex about these; 
//! they are, by and large, a subset of Bevy's generic Curves. 
//! 
//! For Utility AI purposes, all Curves have a unit interval domain (i.e. 0.0 to 1.0), 
//! and a range of values that is ALSO a unit interval (visually forming a 1x1 square).
//! On top of that most of them are fairly simple and cheap (by necessity).
//! 
//! The main purpose of these Curves is to make Considerations more *expressive*.
//! 
//! A Consideration will give us the current Health, but should the Action score 
//! be higher (e.g. for RecklessAttack) or lower (e.g. for Heal) the higher the 
//! Health value is? Or perhaps the highest score should be around the middle? 
//! And even if higher is better, does 10% higher == 10% better, or is it nonlinear?
//! 
//! Curves provide us with the tool to handle this by mapping from the input to the output smoothly.

use bevy::{math};
use crate::types::{ActionScore, MIN_CONSIDERATION_SCORE, MAX_CONSIDERATION_SCORE};

/// Curve functions suitable for Utility scoring purposes.
/// 
/// A strict subset of Bevy's Curve trait.
/// 
/// All eligible functions must have a unity domain (i.e. <0.0; 1.0>) **AND* an output range 
/// of unity as well, or at least you must be willing to allow them to be clamped to this range 
/// by using the `UtilityCurve::sample_safe(&self, t)` method provided.
/// 
/// The datatype is also fixed to use whatever the ActionScore is implemented as.
pub trait UtilityCurve: math::Curve<ActionScore> {
    /// The interval over which this curve is parametrized.
    /// 
    /// This is the range of values of t where we can sample the curve and receive valid output.
    /// 
    /// **DO NOT** override the default impl here! 
    /// 
    /// If you see a conflicting value in impls - good, that means the trait was implemented 
    /// for something that shouldn't be a UtilityCurve and you spotted it before it became a
    /// major headache.
    fn domain(&self) -> math::curve::Interval {
        math::curve::Interval::UNIT
    }

    /// **IMPORTANT!** Use this method for sampling for Utility purposes.
    /// 
    /// Sample a given point on the curve, clamping **both** the input and output values to a unit square.
    /// This is subtly different from `Curve::sample_clamped()` as that only clamps the *input* value.
    /// 
    /// For Utility purposes, the output must be on the unit interval as well, or you will Cause Trouble.
    fn sample_safe(&self, t: ActionScore) -> ActionScore {
        let clampin = math::curve::Interval::UNIT.clamp(t);
        let raw = self.sample_unchecked(clampin);
        let clampout = raw.clamp(MIN_CONSIDERATION_SCORE, MAX_CONSIDERATION_SCORE);
        clampout
    }
}

/// A curve with a constant, user-defined value. 
/// 
/// Will return the same score when sampled anywhere.
#[derive(Debug)]
pub struct UtilityConstantCurve {
    val: ActionScore
}

impl UtilityConstantCurve {
    /// Create a constant curve, which always produces the given value when sampled.
    /// This constructor is fallible - it will return an error if the constant value 
    /// is outside of the range of values valid for a Utility Curve.
    pub fn new(value: ActionScore) -> Result<Self, ()> {
        match math::curve::Interval::UNIT.contains(value) {
            true => Ok(Self {val: value}),
            false => Err(())
        }
    }

    /// Create a constant curve, which always produces the given value when sampled.
    /// Does not check if the value provided is valid for Utility purposes - this is 
    /// faster, but may cause weirdness if someone feeds in a junk value.
    pub fn new_unchecked(value: ActionScore) -> Self {
        Self { val: value }
    }

    /// Create a constant curve, which always produces the given value when sampled.
    /// Ensures that the constant value is valid by clamping it.
    pub fn new_clamped(value: ActionScore) -> Self {
        Self { val: math::curve::Interval::UNIT.clamp(value) }
    }

    /// Create a constant curve, which always produces the given value when sampled.
    /// 
    /// The value is fully const and always safe to construct (enforced at type-level).
    /// 
    /// The value is effectively calculated as (VAL/256), e.g.
    /// - UtilityStaticConstantCurve<0> will return 0.00, 
    /// - UtilityStaticConstantCurve<64> will return 0.25.
    /// - UtilityStaticConstantCurve<128> will return 0.50.
    /// - UtilityStaticConstantCurve<255> will return 1.00 (special case).
    /// 
    /// This means we can conveniently represent all common predefined values as u8 
    /// easily, at the low, low price of sacrificing 0.99609375 as unrepresentable.
    /// If you are hung up about it, use UtilityConstantCurve for it instead.
    pub const fn new_const(value: u8) -> Self {
        Self {
            val: match value {
                0 => 0.,
                255 => 1.,
                mid => (mid as ActionScore) / 256.
            }
        }
    }
}

impl math::Curve<ActionScore> for UtilityConstantCurve {
    fn domain(&self) -> math::curve::Interval {
        math::curve::Interval::UNIT
    }

    fn sample_unchecked(&self, _: f32) -> ActionScore {
        self.val
    }
}

// Trivial impl since it just marks that we've ensured the invariants hold.
impl UtilityCurve for UtilityConstantCurve {}

// Handy common const-valued curves:
//
/// A curve that always returns zero; can be used to temporarily knock out an Action 
/// for testing/debugging without deleting it from game data outright.
pub const NEVER_CURVE: UtilityConstantCurve = UtilityConstantCurve::new_const(0);
//
/// A curve that always returns full score; mainly useful as a placeholder.
pub const ALWAYS_CURVE: UtilityConstantCurve = UtilityConstantCurve::new_const(255);
//
/// A curve that always returns 0.25; mainly useful as a placeholder.
pub const CONST_QUARTER_CURVE: UtilityConstantCurve = UtilityConstantCurve::new_const(64);
//
/// A curve that always returns 0.50; mainly useful as a placeholder.
pub const CONST_HALF_CURVE: UtilityConstantCurve = UtilityConstantCurve::new_const(128);
//
/// A curve that always returns 0.75; mainly useful as a placeholder.
pub const CONST_THREEQUARTER_CURVE: UtilityConstantCurve = UtilityConstantCurve::new_const(192);

// Borrowing usable Bevy curves for impls.
// This only includes the curves natively outputting in the unit interval.
// We clamp the outputs anyway, but it would be confusing.

// Supported easing curves
impl UtilityCurve for math::curve::CircularInCurve {}
impl UtilityCurve for math::curve::CircularInOutCurve {}
impl UtilityCurve for math::curve::CircularOutCurve {}
impl UtilityCurve for math::curve::CubicInCurve {}
impl UtilityCurve for math::curve::CubicInOutCurve {}
impl UtilityCurve for math::curve::CubicOutCurve {}
impl UtilityCurve for math::curve::ExponentialInCurve {}
impl UtilityCurve for math::curve::ExponentialInOutCurve {}
impl UtilityCurve for math::curve::ExponentialOutCurve {}
impl UtilityCurve for math::curve::LinearCurve {}
impl UtilityCurve for math::curve::QuadraticInCurve {}
impl UtilityCurve for math::curve::QuadraticInOutCurve {}
impl UtilityCurve for math::curve::QuadraticOutCurve {}
impl UtilityCurve for math::curve::QuarticInCurve {}
impl UtilityCurve for math::curve::QuarticInOutCurve {}
impl UtilityCurve for math::curve::QuarticOutCurve {}
impl UtilityCurve for math::curve::QuinticInCurve {}
impl UtilityCurve for math::curve::QuinticInOutCurve {}
impl UtilityCurve for math::curve::QuinticOutCurve {}
impl UtilityCurve for math::curve::SineInCurve {}
impl UtilityCurve for math::curve::SineInOutCurve {}
impl UtilityCurve for math::curve::SineOutCurve {}
impl UtilityCurve for math::curve::SmoothStepCurve {}
impl UtilityCurve for math::curve::SmoothStepInCurve {}
impl UtilityCurve for math::curve::SmoothStepOutCurve {}
impl UtilityCurve for math::curve::SmootherStepCurve {}
impl UtilityCurve for math::curve::SmootherStepInCurve {}
impl UtilityCurve for math::curve::SmootherStepOutCurve {}

// A reverse of any valid curve is still a valid curve
impl<U: UtilityCurve> UtilityCurve for math::curve::ReverseCurve<ActionScore, U> {}

/// Specifies the transform used by the UtilityCurveSampler. 
/// - FORWARD => pass-through to `UtilityCurve::sample_safe()`.
/// - INVERSE => `(1.0 - UtilityCurve::sample_safe())` transform.
#[derive(Debug)]
pub enum CurveSamplerMode {
    FORWARD,
    INVERSE,
}

/// A very common transformation for Utility purposes is inverting a Curve's outputs, 
/// returning `(1. - raw_score)` rather than `raw_score`.
/// 
/// UtilityCurveSampler is a wrapper over a UtilityCurve that captures such transforms.
/// 
/// Note that these samplers themselves fulfil the trait interfaces of Curve and UtilityCurve. 
/// 
/// While it would theoretically be possible to stack them, it would be pretty pointless, 
/// as any pair of CurveSamplerModes either cancel each other out or are pass-through.
/// 
/// This is very similar to Bevy's ReverseCurve<T, C>, but specialized for Utility Curves 
/// in some ways that *might* make it a bit cheaper. Mainly though, const constructors go brr.
#[derive(Debug)]
pub struct UtilityCurveSampler<U: UtilityCurve> {
    curve: U,
    mode: CurveSamplerMode,
}

impl<U: UtilityCurve> UtilityCurveSampler<U> {
    pub const fn new(curve: U, mode: CurveSamplerMode) -> Self {
        Self {
            curve: curve,
            mode: mode,
        }
    }

    pub const fn new_forward(curve: U) -> Self {
        Self {
            curve: curve,
            mode: CurveSamplerMode::FORWARD,
        }
    }

    pub const fn new_inverse(curve: U) -> Self {
        Self {
            curve: curve,
            mode: CurveSamplerMode::INVERSE,
        }
    }
}

impl<U: UtilityCurve> math::Curve<ActionScore> for UtilityCurveSampler<U> {
    fn domain(&self) -> math::curve::Interval {
        math::curve::Interval::UNIT
    }

    fn sample_unchecked(&self, t: f32) -> ActionScore {
        match self.mode {
            CurveSamplerMode::FORWARD => self.curve.sample_safe(t),
            CurveSamplerMode::INVERSE => 1. - self.curve.sample_safe(t),
        }
    }
}

impl<U: UtilityCurve> UtilityCurve for UtilityCurveSampler<U> {}

// We're wrapping all of these in UtilityCurveSamplers even when not really necessary 
// for the sake of more predictable, uniform typing.
pub const CURVE_CONST_ZERO: UtilityCurveSampler<UtilityConstantCurve> = UtilityCurveSampler::new_forward(UtilityConstantCurve::new_const(0));
pub const CURVE_CONST_MAX: UtilityCurveSampler<UtilityConstantCurve> = UtilityCurveSampler::new_forward(UtilityConstantCurve::new_const(255));
pub const CURVE_CONST_HALF: UtilityCurveSampler<UtilityConstantCurve> = UtilityCurveSampler::new_forward(UtilityConstantCurve::new_const(128));
pub const CURVE_LINEAR: UtilityCurveSampler<math::curve::LinearCurve> = UtilityCurveSampler::new_forward(math::curve::LinearCurve {});
pub const CURVE_ANTILINEAR: UtilityCurveSampler<math::curve::LinearCurve> = UtilityCurveSampler::new_inverse(math::curve::LinearCurve {});
pub const CURVE_SQUARE: UtilityCurveSampler<math::curve::QuadraticInCurve> = UtilityCurveSampler::new_forward(math::curve::QuadraticInCurve {});
pub const CURVE_ANTISQUARE: UtilityCurveSampler<math::curve::QuadraticInCurve> = UtilityCurveSampler::new_inverse(math::curve::QuadraticInCurve {});
pub const CURVE_EXPONENTIAL: UtilityCurveSampler<math::curve::ExponentialInCurve> = UtilityCurveSampler::new_forward(math::curve::ExponentialInCurve {});
pub const CURVE_ANTIEXPONENTIAL: UtilityCurveSampler<math::curve::ExponentialInCurve> = UtilityCurveSampler::new_inverse(math::curve::ExponentialInCurve {});
pub const CURVE_SIGMOID: UtilityCurveSampler<math::curve::ExponentialInOutCurve> = UtilityCurveSampler::new_inverse(math::curve::ExponentialInOutCurve {});
pub const CURVE_ANTISIGMOID: UtilityCurveSampler<math::curve::ExponentialInOutCurve> = UtilityCurveSampler::new_inverse(math::curve::ExponentialInOutCurve {});


pub enum SupportedUtilityCurve {
    ConstZero(UtilityCurveSampler<UtilityConstantCurve>),
    ConstMax(UtilityCurveSampler<UtilityConstantCurve>),
    ConstHalf(UtilityCurveSampler<UtilityConstantCurve>),
    Linear(UtilityCurveSampler<math::curve::LinearCurve>),
    AntiLinear(UtilityCurveSampler<math::curve::LinearCurve>),
    Square(UtilityCurveSampler<math::curve::QuadraticInCurve>),
    AntiSquare(UtilityCurveSampler<math::curve::QuadraticInCurve>),
    ExponentialIn(UtilityCurveSampler<math::curve::ExponentialInCurve>),
    AntiExponentialIn(UtilityCurveSampler<math::curve::ExponentialInCurve>),
    Sigmoid(UtilityCurveSampler<math::curve::ExponentialInOutCurve>),
    AntiSigmoid(UtilityCurveSampler<math::curve::ExponentialInOutCurve>),
}

impl math::Curve<ActionScore> for SupportedUtilityCurve {
    fn domain(&self) -> math::curve::Interval {
        math::curve::Interval::UNIT
    }

    fn sample_unchecked(&self, t: f32) -> ActionScore {
        match self {
            Self::ConstZero(c) => c.sample_unchecked(t),
            Self::ConstMax(c) => c.sample_unchecked(t),
            Self::ConstHalf(c) => c.sample_unchecked(t),
            Self::Linear(c) => c.sample_unchecked(t),
            Self::AntiLinear(c) => c.sample_unchecked(t),
            Self::Square(c) => c.sample_unchecked(t),
            Self::AntiSquare(c) => c.sample_unchecked(t),
            Self::ExponentialIn(c) => c.sample_unchecked(t),
            Self::AntiExponentialIn(c) => c.sample_unchecked(t),
            Self::Sigmoid(c) => c.sample_unchecked(t),
            Self::AntiSigmoid(c) => c.sample_unchecked(t),
        }
    }
}

impl UtilityCurve for SupportedUtilityCurve {}


impl TryFrom<&str> for SupportedUtilityCurve {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        resolve_curve_from_name(value).ok_or(())
    }
}

impl TryFrom<&String> for SupportedUtilityCurve {
    type Error = ();

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        resolve_curve_from_name(value.as_str()).ok_or(())
    }
}

/// Retrieves a Utility curve based on a string(-ish) key.
pub fn resolve_curve_from_name<S: std::borrow::Borrow<str>>(curve_name: S) -> Option<SupportedUtilityCurve> {
    match curve_name.borrow() {
        "ConstZero" => Some(SupportedUtilityCurve::ConstZero(CURVE_CONST_ZERO)),
        "ConstMax" => Some(SupportedUtilityCurve::ConstMax(CURVE_CONST_MAX)),
        "ConstHalf" => Some(SupportedUtilityCurve::ConstHalf(CURVE_CONST_HALF)),
        "Linear" => Some(SupportedUtilityCurve::Linear(CURVE_LINEAR)),
        "AntiLinear" => Some(SupportedUtilityCurve::AntiLinear(CURVE_ANTILINEAR)),
        "Square" => Some(SupportedUtilityCurve::Square(CURVE_SQUARE)),
        "AntiSquare" => Some(SupportedUtilityCurve::AntiSquare(CURVE_ANTISQUARE)),
        "ExponentialIn" => Some(SupportedUtilityCurve::ExponentialIn(CURVE_EXPONENTIAL)),
        "AntiExponentialIn" => Some(SupportedUtilityCurve::AntiExponentialIn(CURVE_ANTIEXPONENTIAL)),
        "Sigmoid" => Some(SupportedUtilityCurve::Sigmoid(CURVE_SIGMOID)),
        "AntiSigmoid" => Some(SupportedUtilityCurve::AntiSigmoid(CURVE_ANTISIGMOID)),
        _ => None,
    }
}
