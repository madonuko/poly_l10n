use crate::LanguageIdentifier;
use isolang::Language;
use itertools::Itertools;

/// [`crate::Rulebook`] function for the default recommended rule(s).
#[inline]
pub fn default_rulebook(l: &LanguageIdentifier) -> Vec<LanguageIdentifier> {
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
            }),*])
        };
        (@$rule:literal) => { concat!("cannot parse ", $rule) };
        (@$rule:expr) => { &format!("cannot parse {}", $rule) };
    }

    // double line → macro
    // single line → written / standard / main
    match lang {
        Language::Ara | Language::Arb if l.variants().len() == 0 => {
            //    ═══             ───
            rules!["ar-AE", "ara-AE", "arb-AE"];
        }
        Language::Zho | Language::Cmn => match l.script {
            //    ═══             ───
            Some(s) if s.as_str().eq_ignore_ascii_case("Hans") => {
                rules!["zh-Hans-CN", "zho-Hans-CN", "cmn-Hans-CN", "zh-Hant"];
            }
            Some(s) if s.as_str().eq_ignore_ascii_case("Hant") => {
                rules!["zh-Hant-TW", "zho-Hant-TW", "cmn-Hant-TW", "zh-Hans"];
            }
            #[allow(unused_variables)]
            Some(script) => {
                #[cfg(feature = "tracing")]
                tracing::warn!(?l, ?script, "unknown script for zho");
            }
            None => match l.region.as_ref().map(unic_langid::subtags::Region::as_str) {
                Some("CN" | "SG") => rules!["zh-Hans-CN", "zho-Hans-CN", "cmn-Hans-CN"],
                Some("TW") => rules!["zh-Hant-TW", "zho-Hant-TW", "cmn-Hant-TW"],
                Some("HK" | "MO") => rules![
                    "zh-Hant-HK",
                    "zho-Hant-HK",
                    "cmn-Hant-HK",
                    "zh-Hant-TW",
                    "zho-Hant-TW",
                    "cmn-Hant-TW"
                ],
                Some(region) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(region, "unknown zh region");
                    rules![format!("zh-Hans-{region}"), format!("zh-Hant-{region}")];
                }
                None => rules![
                    "zh-Hans-CN",
                    "zho-Hans-CN",
                    "cmn-Hans-CN",
                    "zh-Hant-TW",
                    "zho-Hant-TW",
                    "cmn-Hant-TW"
                ],
            },
        },
        _ => {}
    }

    if l.language.as_str().len() == 3 {
        #[cfg(feature = "tracing")]
        tracing::trace!(?l, "fallback unknown lang");
        if let Some(two) = lang.to_639_1() {
            rules![two];
        }
    } else if l.language.as_str().len() == 2 {
        #[cfg(feature = "tracing")]
        tracing::trace!(?l, "fallback unknown lang");
        rules![lang.to_639_3()];
    }

    let new_rules = rules.iter().flat_map(find_rules_omit_optparts);
    rules.extend_from_slice(&new_rules.filter(|r| rules.contains(r)).collect_vec());

    rules
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
