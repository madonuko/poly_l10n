use crate::LanguageIdentifier;
use isolang::Language;
use itertools::Itertools;

/// [`crate::Rulebook`] function for the default recommended rule(s).
#[inline]
pub fn default_rulebook(l: &LanguageIdentifier) -> Vec<LanguageIdentifier> {
    let Some(lang) = langid_to_isolang(l) else {
        return vec![];
    };

    let mut rules: Vec<LanguageIdentifier> = vec![];

    macro_rules! rules {
        ($($rule:expr),*$(,)?) => {
            rules.extend_from_slice(&[$({
                let rule = $rule;
                rule.parse().expect(rules!(@rule))
            }),*])
        };
        (@$rule:literal) => { concat!("cannot parse ", $rule) };
        (@$rule:expr) => { &format!("cannot parse {}", $rule) };
    }

    if l.language.as_str().len() == 2 {
        #[cfg(feature = "tracing")]
        tracing::trace!(?l, "fallback unknown lang");
        if let Some(two) = lang.to_639_1() {
            rules![two];
        }
    } else if l.language.as_str().len() == 3 {
        #[cfg(feature = "tracing")]
        tracing::trace!(?l, "fallback unknown lang");
        rules![lang.to_639_3()];
    }

    #[cfg(feature = "per_lang_default_rules")]
    #[allow(clippy::indexing_slicing)]
    if let Some(f) = &crate::per_lang_default_rules::LANG_RULES[lang as usize] {
        rules.extend_from_slice(&f(l, &lang));
    }

    let new_rules = rules.iter().flat_map(find_rules_omit_optparts);
    let new_rules = new_rules.unique().collect_vec();
    #[cfg(feature = "tracing")]
    tracing::trace!(?rules, ?new_rules);
    rules.extend_from_slice(&new_rules);

    rules
}

fn langid_to_isolang(l: &LanguageIdentifier) -> Option<Language> {
    let lang = match l.language.as_str().len() {
        2 => Language::from_639_1(l.language.as_str()),
        3 => Language::from_639_3(l.language.as_str()),
        #[allow(unused_variables)]
        len => {
            #[cfg(feature = "tracing")]
            tracing::error!(?l, len, "invalid language code, expected length of 2 or 3");
            return None;
        }
    };
    #[cfg(feature = "tracing")]
    if lang.is_none() {
        tracing::error!(?l, "invalid language code, fail to parse with `isolang`");
    }
    lang
}

/// Generate a list of [`LanguageIdentifier`] without `script`, `region` and/or `variants` from
/// the given `rule`.
///
/// This gives all possible combinations of [`LanguageIdentifier`] with the given `rule` without
/// the optional parts.
#[allow(clippy::arithmetic_side_effects)]
#[inline]
fn find_rules_omit_optparts(rule: &LanguageIdentifier) -> impl Iterator<Item = LanguageIdentifier> {
    let (ii, jj, kk) = (
        usize::from(rule.script.is_some()) + 1,
        usize::from(rule.region.is_some()) + 1,
        rule.variants().len(),
    );
    let k = (0..kk)
        .map(|_| [false, true].into_iter())
        .multi_cartesian_product();
    itertools::iproduct!(0..ii, 0..jj, k).filter_map(move |(i, j, v)| {
        if i == ii - 1 && j == jj - 1 && v.iter().all(|&b| b) {
            // equal orig
            return None;
        }
        let mut r = rule.clone();
        if i == 0 {
            r.script = None;
        }
        if j == 0 {
            r.region = None;
        }
        r.clear_variants();
        r.set_variants(
            &v.into_iter()
                .enumerate()
                .filter_map(|(i, k)| k.then_some(i))
                .map(|i| rule.variants().nth(i).unwrap().to_owned())
                .collect_vec(),
        );
        Some(r)
    })
}
