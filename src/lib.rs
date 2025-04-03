//! `poly_l10n`: Handle locali(s|z)ations the correct way
//!
//! ## Intentions
//!
//! See <https://blog.fyralabs.com/advice-on-internationalization/#language-fallbacks>.
//!
//! In short, this crate handles language fallbacks and detect system languages *the correct way*.
//!
//! Get started by [`LocaleFallbackSolver`] and [`langid!`].
//!
//! ## 📃 License
//!
//! `GPL-3.0-or-later`
//!
//!    Copyright (C) 2025  madonuko <mado@fyralabs.com> <madonuko@outlook.com>
//!
//!    This program is free software: you can redistribute it and/or modify
//!    it under the terms of the GNU General Public License as published by
//!    the Free Software Foundation, either version 3 of the License, or
//!    (at your option) any later version.
//!
//!    This program is distributed in the hope that it will be useful,
//!    but WITHOUT ANY WARRANTY; without even the implied warranty of
//!    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//!    GNU General Public License for more details.
//!
//!    You should have received a copy of the GNU General Public License
//!    along with this program.  If not, see <https://www.gnu.org/licenses/>.

mod default_rulebook;
pub mod macros;

use std::rc::Rc;

use itertools::Itertools;
pub use unic_langid::{self, LanguageIdentifier};

/// Entry point of `poly_l10n`.
///
/// # Examples
/// ```
/// let solver = poly_l10n::LocaleFallbackSolver::<poly_l10n::Rulebook>::default();
/// assert_eq!(solver.solve_locale(poly_l10n::langid!("arb")), poly_l10n::langid!["ar-AE", "ara-AE", "arb-AE", "ar", "ara", "arb"]);
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct LocaleFallbackSolver<R: for<'a> PolyL10nRulebook<'a> = Rulebook> {
    pub rulebook: R,
}

impl<R: for<'a> PolyL10nRulebook<'a>> LocaleFallbackSolver<R> {
    /// Find alternative fallbacks for the given `locale` as specified by the `rulebook`. This
    /// operation is recursive and expensive.
    ///
    /// ```
    /// let solver = poly_l10n::LocaleFallbackSolver::<poly_l10n::Rulebook>::default();
    /// assert_eq!(solver.solve_locale(poly_l10n::langid!("arb")), poly_l10n::langid!["ar-AE", "ara-AE", "arb-AE", "ar", "ara", "arb"]);
    /// ```
    pub fn solve_locale<L: AsRef<LanguageIdentifier>>(&self, locale: L) -> Vec<LanguageIdentifier> {
        use std::hash::{Hash, Hasher};
        let locale = locale.as_ref();
        let mut locales = self.rulebook.find_fallback_locale(locale).collect_vec();
        let h = |l: &LanguageIdentifier| {
            let mut hasher = std::hash::DefaultHasher::default();
            l.hash(&mut hasher);
            hasher.finish()
        };
        let mut locale_hashes = locales.iter().map(h).collect_vec();
        let mut old_len = 0;
        while old_len != locales.len() {
            #[allow(clippy::indexing_slicing)]
            let new_locales = locales[old_len..]
                .iter()
                .flat_map(|locale| {
                    self.rulebook.find_fallback_locale(locale).chain(
                        self.rulebook
                            .find_fallback_locale_ref(locale)
                            .map(Clone::clone),
                    )
                })
                .filter(|l| !locale_hashes.contains(&h(l)))
                .unique()
                .collect_vec();
            old_len = locales.len();
            locales.extend_from_slice(&new_locales);
            locale_hashes.extend(new_locales.iter().map(h));
        }
        locales.into_iter().unique().collect_vec()
    }
}

/// Rulebook trait.
///
/// A rulebook is a set of rules for [`LocaleFallbackSolver`]. The solver obtains the list of
/// fallback locales from the rules in the solver's rulebook.
///
/// The default rulebook is [`Rulebook`] and you may create a solver with it using:
///
/// ```
/// poly_l10n::LocaleFallbackSolver::<poly_l10n::Rulebook>::default()
/// # ;
/// ```
///
/// With that being said, a custom tailor-made rulebook is possible by implementing this trait for
/// a new struct.
///
/// # Implementation
/// Only one of [`PolyL10nRulebook::find_fallback_locale`] and
/// [`PolyL10nRulebook::find_fallback_locale_ref`] SHOULD be implemented. Note that for the latter,
/// [`LocaleFallbackSolver`] will clone the items in the returned iterator, so there are virtually
/// no performance difference between the two.
///
/// If both functions are implemented, the solver will [`Iterator::chain`] them together.
pub trait PolyL10nRulebook<'s> {
    fn find_fallback_locale(
        &self,
        _: &LanguageIdentifier,
    ) -> impl Iterator<Item = LanguageIdentifier> {
        std::iter::empty()
    }

    fn find_fallback_locale_ref(
        &'s self,
        _: &LanguageIdentifier,
    ) -> impl Iterator<Item = &'s LanguageIdentifier> {
        std::iter::empty()
    }
}

// NOTE: rust disallows multiple blanket impls, so unfortunately we need to choose one
/*
impl<'s, M> PolyL10nRulebook<'s> for M
where
    M: for<'a> std::ops::Index<&'a LanguageIdentifier, Output = LanguageIdentifier>,
{
    fn find_fallback_locale(
        &'s self,
        locale: &LanguageIdentifier,
    ) -> impl Iterator<Item = &'s LanguageIdentifier> {
        std::iter::once(&self[locale])
    }
}
*/

impl<'s, M, LS: 's> PolyL10nRulebook<'s> for M
where
    M: for<'a> std::ops::Index<&'a LanguageIdentifier, Output = LS>,
    &'s LS: IntoIterator<Item = &'s LanguageIdentifier>,
{
    fn find_fallback_locale_ref(
        &'s self,
        locale: &LanguageIdentifier,
    ) -> impl Iterator<Item = &'s LanguageIdentifier> {
        (&self[locale]).into_iter()
    }
}

pub type FnRules = Vec<Box<dyn Fn(&LanguageIdentifier) -> Vec<LanguageIdentifier>>>;

/// A set of rules that govern how [`LocaleFallbackSolver`] should handle fallbacks.
///
/// [`Rulebook<A>`], regardless of type `A`, stores the rules as [`FnRules`], a vector of boxed
/// `dyn Fn(&LanguageIdentifier) -> Vec<LanguageIdentifier>`. Therefore, the actual correct name of
/// this struct should be something along the lines of `FnsRulebook`.
///
/// Obviously this rulebook can be used with the solver because it implements [`PolyL10nRulebook`].
///
/// In addition, the default rulebook [`Rulebook::default()`] can and probably should be used for
/// most situations you ever need to deal with.
pub struct Rulebook<A = ()> {
    pub rules: FnRules,
    pub owned_values: A,
}

impl<A: std::fmt::Debug> std::fmt::Debug for Rulebook<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rulebook")
            .field("owned_values", &self.owned_values)
            .field("rules", &PseudoFnRules::from(&self.rules))
            .finish_non_exhaustive()
    }
}
/// Used for implementing [`Debug`] for [`Rulebook`].
struct PseudoFnRules {
    len: usize,
}
impl From<&FnRules> for PseudoFnRules {
    fn from(value: &FnRules) -> Self {
        Self { len: value.len() }
    }
}
impl std::fmt::Debug for PseudoFnRules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FnRules")
            .field("len", &self.len)
            .finish_non_exhaustive()
    }
}

impl<A> PolyL10nRulebook<'_> for Rulebook<A> {
    fn find_fallback_locale(
        &self,
        locale: &LanguageIdentifier,
    ) -> impl Iterator<Item = LanguageIdentifier> {
        self.rules.iter().flat_map(|f| f(locale))
    }
}

impl Rulebook<Rc<Vec<Rulebook>>> {
    /// Combine multiple rulebooks into one.
    ///
    /// See also: [`Self::from_ref_rulebooks`].
    ///
    /// # Examples
    /// ```
    /// let rb1 = poly_l10n::Rulebook::from_fn(|l| {
    ///   let mut l = l.clone();
    ///   l.script = None;
    ///   vec![l]
    /// });
    /// let rb2 = poly_l10n::Rulebook::from_fn(|l| {
    ///   let mut l = l.clone();
    ///   l.region = None;
    ///   vec![l]
    /// });
    /// let rulebook = poly_l10n::Rulebook::from_rulebooks([rb1, rb2].into_iter());
    /// let solv = poly_l10n::LocaleFallbackSolver { rulebook };
    ///
    /// assert_eq!(
    ///   solv.solve_locale(poly_l10n::langid!["zh-Hant-HK"]),
    ///   poly_l10n::langid!["zh-HK", "zh-Hant", "zh"]
    /// );
    /// ```
    pub fn from_rulebooks<I: Iterator<Item = Rulebook>>(rulebooks: I) -> Self {
        let mut new = Self {
            owned_values: Rc::new(rulebooks.collect_vec()),
            rules: vec![],
        };
        let owned_values = Rc::clone(&new.owned_values);
        new.rules = vec![Box::new(move |l: &LanguageIdentifier| {
            owned_values
                .iter()
                .flat_map(|rulebook| rulebook.find_fallback_locale(l).collect_vec())
                .collect()
        })];
        new
    }
}
impl<RR, R> Rulebook<(Rc<Vec<RR>>, std::marker::PhantomData<R>)>
where
    RR: AsRef<Rulebook<R>> + 'static,
{
    /// Combine multiple rulebooks into one. Each given rulebook `r` must implement
    /// [`AsRef::as_ref`].
    ///
    /// For the owned version, see [`Self::from_rulebooks`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::rc::Rc;
    /// let rb1 = poly_l10n::Rulebook::from_fn(|l| {
    ///   let mut l = l.clone();
    ///   l.script = None;
    ///   vec![l]
    /// });
    /// let rb2 = poly_l10n::Rulebook::from_fn(|l| {
    ///   let mut l = l.clone();
    ///   l.region = None;
    ///   vec![l]
    /// });
    /// let (rb1, rb2) = (Rc::new(rb1), Rc::new(rb2));
    /// let rulebook = poly_l10n::Rulebook::from_ref_rulebooks([rb1, rb2].iter().cloned());
    /// let solv = poly_l10n::LocaleFallbackSolver { rulebook };
    ///
    /// assert_eq!(
    ///   solv.solve_locale(poly_l10n::langid!["zh-Hant-HK"]),
    ///   poly_l10n::langid!["zh-HK", "zh-Hant", "zh"]
    /// );
    /// ```
    pub fn from_ref_rulebooks<I: Iterator<Item = RR>>(rulebooks: I) -> Self {
        let mut new = Self {
            owned_values: (Rc::new(rulebooks.collect_vec()), std::marker::PhantomData),
            rules: vec![],
        };
        let owned_values = Rc::clone(&new.owned_values.0);
        new.rules = vec![Box::new(move |l: &LanguageIdentifier| {
            (owned_values.iter())
                .flat_map(|rulebook| rulebook.as_ref().find_fallback_locale(l).collect_vec())
                .collect()
        })];
        new
    }
}

impl Rulebook {
    #[must_use]
    pub fn from_fn<F: Fn(&LanguageIdentifier) -> Vec<LanguageIdentifier> + 'static>(f: F) -> Self {
        Self {
            rules: vec![Box::new(f)],
            owned_values: (),
        }
    }
    #[must_use]
    pub const fn from_fns(rules: FnRules) -> Self {
        Self {
            rules,
            owned_values: (),
        }
    }
    /// Convert a map (or anything that impl [`std::ops::Index<&LanguageIdentifier>`]) into
    /// a rulebook.
    ///
    /// The output of the map must implement [`IntoIterator<Item = &LanguageIdentifier>`].
    ///
    /// While any valid arguments to this constructor are guaranteed to satisfy the trait
    /// [`PolyL10nRulebook`], it could be useful to convert them to rulebooks, e.g. to combine
    /// multiple rulebooks using [`Self::from_rulebooks`].
    pub fn from_map<M, LS>(map: M) -> Self
    where
        M: for<'a> std::ops::Index<&'a LanguageIdentifier, Output = LS> + 'static,
        for<'b> &'b LS: IntoIterator<Item = &'b LanguageIdentifier>,
    {
        Self::from_fn(move |l| map[l].into_iter().cloned().collect())
    }
}

// TODO: rules?
impl Default for Rulebook {
    fn default() -> Self {
        Self::from_fn(default_rulebook::default_rulebook)
    }
}
