//! # decimix-finance
//!
//! Domain types for trading, built on [`decimix`]: [`Price`], [`Qty`],
//! [`DeltaQty`], [`Amt`] and [`Percentage`].
//!
//! Each wraps a 19-decimal-place number, and they can't be mixed by
//! accident: adding a price to a quantity doesn't compile.
//!
//! The names and meanings follow the FIX 5 datatypes, so a FIX field maps to
//! the type of the same name:
//!
//! | FIX datatype | Type             | Meaning                                  |
//! |--------------|------------------|------------------------------------------|
//! | `Price`      | [`Price`]        | a price; may be negative                 |
//! | `Qty`        | [`Qty`]          | a quantity; here, never negative         |
//! | —            | [`DeltaQty`]     | a quantity that may be negative          |
//! | `Amt`        | [`Amt`]          | typically price × quantity: a cash value |
//! | `Percentage` | [`Percentage`]   | a fraction: 0.05 means 5%                |
//!
//! FIX has no separate signed-quantity type; [`DeltaQty`] is this crate's
//! addition, so that [`Qty`] can promise it is never negative. FIX's
//! `PriceOffset` is not provided yet.
//!
//! ## Quantities: [`Qty`] and [`DeltaQty`]
//!
//! A quantity comes in two forms. [`Qty`] can never be negative: an order
//! size, a fill, a lot. [`DeltaQty`] is a quantity that can be: a position,
//! a net fill, a change in either. Both are the same kind of thing, so the rules
//! for quantities apply to both. The [`Quantity`] trait expresses that:
//! price × quantity is defined once, for either form, rather than once per
//! sign.
//!
//! ```
//! use decimix::{dec, udec};
//! use decimix_finance::{DeltaQty, Qty};
//!
//! let ordered = Qty::new(udec!(1000));
//! let filled = Qty::new(udec!(250));
//! let leaves: Qty = ordered - filled; // panics if it would go negative
//! assert_eq!(leaves, Qty::new(udec!(750)));
//! let delta: DeltaQty = filled.signed_sub(ordered); // may be negative
//! assert_eq!(delta, DeltaQty::new(dec!(-750)));
//! assert_eq!(delta.unsigned_abs(), leaves);
//! ```
//!
//! ## Price × quantity is an amount
//!
//! Multiplying a price by either form of quantity gives an [`Amt`]: the cash
//! value (the consideration), negative when the price or quantity is. Like every decimal product,
//! it names its rounding.
//!
//! ```
//! use decimix::{Round, dec, udec};
//! use decimix_finance::{DeltaQty, Amt, Price, Qty, Percentage};
//!
//! let px = Price::new(dec!(113.725));
//!
//! let bought: Amt = px.mul(Qty::new(udec!(250)), Round::HalfEven);
//! assert_eq!(bought, Amt::new(dec!(28431.25)));
//!
//! let short: Amt = px.mul(DeltaQty::new(dec!(-250)), Round::HalfEven);
//! assert_eq!(short, Amt::new(dec!(-28431.25)));
//!
//! let fee = bought.mul(Percentage::new(dec!(0.0002)), Round::HalfEven);
//! assert_eq!(fee, Amt::new(dec!(5.68625)));
//! ```
//!
//! ## What doesn't compile, on purpose
//!
//! A price times a quantity is neither a quantity nor a price:
//!
//! ```compile_fail
//! use decimix::{Round, dec, udec};
//! use decimix_finance::{DeltaQty, Price, Qty};
//! let px = Price::new(dec!(1.5));
//! let a: DeltaQty = px.mul(Qty::new(udec!(2)), Round::HalfEven);
//! ```
//!
//! ```compile_fail
//! use decimix::{Round, dec, udec};
//! use decimix_finance::{Price, Qty};
//! let px = Price::new(dec!(1.5));
//! let p: Price = px.mul(Qty::new(udec!(2)), Round::HalfEven);
//! ```
//!
//! Price × price, and price × amount, mean nothing:
//!
//! ```compile_fail
//! use decimix::{Round, dec};
//! use decimix_finance::Price;
//! let px = Price::new(dec!(1.5));
//! let x = px.mul(px, Round::HalfEven);
//! ```
//!
//! ```compile_fail
//! use decimix::{Round, dec};
//! use decimix_finance::{Amt, Price};
//! let x = Price::new(dec!(1.5)).mul(Amt::new(dec!(2)), Round::HalfEven);
//! ```
//!
//! Different kinds don't add, and signed and unsigned quantities don't mix
//! without saying how:
//!
//! ```compile_fail
//! use decimix::{dec, udec};
//! use decimix_finance::{Price, Qty};
//! let x = Price::new(dec!(1.5)) + Qty::new(udec!(2));
//! ```
//!
//! ```compile_fail
//! use decimix::{dec, udec};
//! use decimix_finance::{DeltaQty, Qty};
//! let x = DeltaQty::new(dec!(-1)) + Qty::new(udec!(2));
//! ```
//!
//! ```compile_fail
//! use decimix::udec;
//! use decimix_finance::Qty;
//! let q = -Qty::new(udec!(2));
//! ```
//!
//! Domain types have no `f64` methods; floats go through the base type, where
//! a lint can catch them:
//!
//! ```compile_fail
//! use decimix::dec;
//! use decimix_finance::Price;
//! let f = Price::new(dec!(1.5)).to_f64_lossy();
//! ```
#![no_std]
#![warn(missing_docs)]

use decimix::{Dec19, Fixed19, OutOfRange, ProductSum, Round, UDec19, newtype};

newtype! {
  /// A price. May be negative (spreads, some futures and power markets).
  /// FIX datatype `Price`.
  pub struct Price(Dec19);
}

newtype! {
  /// A quantity that can't be negative: an order size, a fill, a lot. FIX
  /// datatype `Qty`.
  ///
  /// For a quantity that can be negative, use [`DeltaQty`].
  pub struct Qty(UDec19);
}

newtype! {
  /// A quantity that can be negative: a position, a net fill, a change in
  /// either. FIX has no datatype for this; it uses `Qty` for both.
  ///
  /// The same kind of thing as [`Qty`], so it takes part in the same
  /// products (see [`Quantity`]).
  pub struct DeltaQty(Dec19);
}

newtype! {
  /// An amount of money: typically price × quantity (the consideration),
  /// or a fee, a P&L. FIX datatype `Amt`.
  pub struct Amt(Dec19);
}

newtype! {
  /// A percentage, held as a fraction: 0.05 means 5%, 0.9525 means 95.25%.
  /// FIX datatype `Percentage`. For fee rates, FX rates and other ratios.
  pub struct Percentage(Dec19);
}

mod sealed {
  pub trait Sealed {}
  impl Sealed for super::Qty {
  }
  impl Sealed for super::DeltaQty {
  }
}

/// Either form of quantity: [`Qty`] (never negative) or [`DeltaQty`] (may be).
///
/// Products are defined on the kind of value, not its sign, so price ×
/// quantity is written once and works for both. Sealed.
pub trait Quantity: Copy + sealed::Sealed {
  /// The plain value, signed or not.
  type Base: Fixed19;

  /// The plain value.
  fn to_base(self) -> Self::Base;
}

impl Quantity for Qty {
  type Base = UDec19;

  fn to_base(self) -> UDec19 {
    self.get()
  }
}

impl Quantity for DeltaQty {
  type Base = Dec19;

  fn to_base(self) -> Dec19 {
    self.get()
  }
}

impl Price {
  /// Price × quantity, rounded to 19 places with `mode`. `qty` may be a
  /// [`Qty`] or a [`DeltaQty`]; the result is negative when the quantity is.
  ///
  /// # Panics
  ///
  /// On overflow. See [`checked_mul`](Self::checked_mul).
  #[track_caller]
  pub fn mul<Q: Quantity>(self, qty: Q, mode: Round) -> Amt {
    Amt::new(self.get().mul(qty.to_base(), mode))
  }

  /// Price × quantity, rounded to 19 places with `mode`, or `None` on
  /// overflow.
  #[must_use]
  pub fn checked_mul<Q: Quantity>(self, qty: Q, mode: Round) -> Option<Amt> {
    self.get().checked_mul(qty.to_base(), mode).map(Amt::new)
  }

  /// The volume-weighted average price of some fills: Σ(price × quantity) ÷
  /// Σ quantity, rounded once with `mode`.
  ///
  /// `None` if there are no fills, the total quantity is zero, or a total
  /// overflows.
  ///
  /// ```
  /// use decimix::{Round, dec, udec};
  /// use decimix_finance::{Price, Qty};
  ///
  /// let fills = [
  ///   (Price::new(dec!(113.725)), Qty::new(udec!(100))),
  ///   (Price::new(dec!(113.73)), Qty::new(udec!(250))),
  /// ];
  /// let vwap = Price::vwap(fills, Round::HalfEven);
  /// assert_eq!(vwap, Some(Price::new(dec!(113.7285714285714285714))));
  /// ```
  #[must_use]
  pub fn vwap<I>(fills: I, mode: Round) -> Option<Price>
  where
    I: IntoIterator<Item = (Price, Qty)>,
  {
    let mut acc = ProductSum::new();
    let mut total = UDec19::ZERO;
    for (px, qty) in fills {
      acc.add(px.get(), qty.get());
      total = total.checked_add(qty.get())?;
    }
    if total.is_zero() {
      return None;
    }
    acc.div(total, mode).ok().map(Price::new)
  }
}

impl Amt {
  /// Amount × percentage (a fee, an FX conversion), rounded to 19 places
  /// with `mode`.
  ///
  /// # Panics
  ///
  /// On overflow.
  #[track_caller]
  pub fn mul(self, pct: Percentage, mode: Round) -> Amt {
    Amt::new(self.get().mul(pct.get(), mode))
  }

  /// Σ(price × quantity), with every product kept exactly and one rounding
  /// at the end. With [`DeltaQty`] quantities this is the net amount
  /// (buys positive, sells negative).
  pub fn sum_products<I, Q>(fills: I, mode: Round) -> Result<Amt, OutOfRange>
  where
    I: IntoIterator<Item = (Price, Q)>,
    Q: Quantity,
  {
    let mut acc = ProductSum::new();
    for (px, qty) in fills {
      acc.add(px.get(), qty.to_base());
    }
    acc.finish(mode).map(Amt::new)
  }
}

impl Qty {
  /// `self - other` as a signed [`DeltaQty`], which may be negative.
  ///
  /// # Panics
  ///
  /// If the difference is outside [`DeltaQty`]'s range (only possible for
  /// quantities above about 1.7 × 10¹⁹).
  #[track_caller]
  pub fn signed_sub(self, other: Qty) -> DeltaQty {
    DeltaQty::new(self.get().signed_sub(other.get()))
  }

  /// The same quantity as a signed [`DeltaQty`].
  ///
  /// # Panics
  ///
  /// Above about 1.7 × 10¹⁹, which [`DeltaQty`] can't hold.
  #[track_caller]
  pub fn to_delta_qty(self) -> DeltaQty {
    DeltaQty::try_from(self).expect("Qty too large for DeltaQty")
  }
}

impl DeltaQty {
  /// The size of the quantity, whichever its sign. Always fits.
  pub fn unsigned_abs(self) -> Qty {
    let raw = self.get().to_raw().unsigned_abs();
    Qty::new(UDec19::from_raw(raw))
  }
}

impl TryFrom<Qty> for DeltaQty {
  type Error = OutOfRange;

  /// Fails only for quantities above about 1.7 × 10¹⁹.
  fn try_from(qty: Qty) -> Result<Self, OutOfRange> {
    Dec19::try_from(qty.get()).map(DeltaQty::new)
  }
}

impl TryFrom<DeltaQty> for Qty {
  type Error = OutOfRange;

  /// Fails if the quantity is negative.
  fn try_from(delta: DeltaQty) -> Result<Self, OutOfRange> {
    UDec19::try_from(delta.get()).map(Qty::new)
  }
}
