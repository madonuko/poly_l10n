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

pub mod macros;

use std::rc::Rc;

pub use unic_langid::{self, LanguageIdentifier};

/// Entry point of `poly_l10n`.
///
/// # Examples
/// ```
/// let solver = poly_l10n::LocaleFallbackSolver::<poly_l10n::Rulebook>::default();
/// assert_eq!(solver.solve_locale(poly_l10n::langid!("arb")), poly_l10n::langid!["ar-AE", "arb-AE", "ara", "ar"]);
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
    /// assert_eq!(solver.solve_locale(poly_l10n::langid!("arb")), poly_l10n::langid!["ar-AE", "arb-AE", "ara", "ar"]);
    /// ```
    pub fn solve_locale<L: AsRef<LanguageIdentifier>>(&self, locale: L) -> Vec<LanguageIdentifier> {
        let locale = locale.as_ref();
        let mut locales = self
            .rulebook
            .find_fallback_locale(locale)
            .collect::<Vec<_>>();
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
                .filter(|l| !locales.contains(l))
                .collect::<Vec<_>>();
            old_len = locales.len();
            locales.extend_from_slice(&new_locales);
        }
        locales
    }
}

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

pub struct Rulebook<A = ()> {
    pub rules: FnRules,
    pub owned_values: A,
}

impl<A: std::any::Any> PolyL10nRulebook<'_> for Rulebook<A> {
    fn find_fallback_locale(
        &self,
        locale: &LanguageIdentifier,
    ) -> impl Iterator<Item = LanguageIdentifier> {
        self.rules.iter().flat_map(|f| f(locale))
    }
}

impl Rulebook<Rc<Vec<Rulebook>>> {
    pub fn from_rulebooks<I: Iterator<Item = Rulebook>>(rulebooks: I) -> Self {
        let mut new = Self {
            owned_values: Rc::new(rulebooks.collect::<Vec<_>>()),
            rules: vec![],
        };
        let owned_values = Rc::clone(&new.owned_values);
        new.rules = vec![Box::new(move |l: &LanguageIdentifier| {
            owned_values
                .iter()
                .flat_map(|rulebook| rulebook.find_fallback_locale(l).collect::<Vec<_>>())
                .collect()
        })];
        new
    }
}

impl Rulebook {
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
    pub const fn from_map<M, LS>(map: M) -> M
    where
        M: for<'a> std::ops::Index<&'a LanguageIdentifier, Output = LS>,
        for<'b> &'b LS: IntoIterator<Item = &'b LanguageIdentifier>,
    {
        map
    }
}

// TODO: rules?
impl Default for Rulebook {
    fn default() -> Self {
        Self::from_fn(|l| {
            let Some(lang) = (match l.language.as_str().len() {
                2 => isolang::Language::from_639_1(l.language.as_str()),
                3 => isolang::Language::from_639_3(l.language.as_str()),
                #[allow(unused_variables)]
                len => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(?l, len, "invalid language code, expected length of 2 or 3");
                    return vec![];
                }
            }) else {
                #[cfg(feature = "tracing")]
                tracing::error!(?l, "invalid language code, fail to parse with `isolang`");
                return vec![];
            };

            let mut rules: Vec<LanguageIdentifier> = vec![];

            macro_rules! rules {
                ($($rule:expr),*$(,)?) => {
                    rules.extend_from_slice(&[$({
                        let rule = $rule;
                        rule.parse().expect(rules!(@rule))
                    }),*]);
                };
                (@$rule:literal) => { concat!("cannot parse ", $rule) };
                (@$rule:expr) => { &format!("cannot parse {}", $rule) };
            }

            match lang.to_639_3() {
                "arb" => {
                    if l.variants().len() == 0 {
                        rules!["ar-AE", "arb-AE"];
                    } else {
                        rules!["ar", "arb"];
                    }
                }
                "zho" => match l.script {
                    Some(s) if s.as_str().eq_ignore_ascii_case("Hans") => todo!(),
                    Some(s) if s.as_str().eq_ignore_ascii_case("Hant") => todo!(),
                    Some(script) => {
                        #[cfg(feature = "tracing")]
                        tracing::warn!(?l, ?script, "unknown script for zho");
                    }
                    None => todo!(),
                },
                _ if l.language.as_str().len() == 3 => {
                    if let Some(two) = lang.to_639_1() {
                        rules![two];
                    }
                }
                three if l.language.as_str().len() == 2 => {
                    rules![three];
                }
                _ => {}
            }

            rules
        })
    }
}
