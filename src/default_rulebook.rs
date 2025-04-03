use crate::LanguageIdentifier;
use isolang::Language;
use itertools::Itertools;

pub const ISOLANG_OVERVIEW_LEN: usize = 7916;

type OptArcFnLangFallbacks = Option<
    std::sync::Arc<dyn Fn(&LanguageIdentifier, &Language) -> Vec<LanguageIdentifier> + Sync + Send>,
>;
type InnerLangRules = [OptArcFnLangFallbacks; ISOLANG_OVERVIEW_LEN];

macro_rules! gen_langrules {
    ($l:ident $lang:ident: $($($Lang:ident)|+ $(if $guard:expr)? => $rule:expr),+$(,)?) => {
        gen_langrules!([$] $l $lang: $($($Lang)|+ $(if $guard)? => $rule),+)
    };
    ([$dollar:tt] $l:ident $lang:ident: $($($Lang:ident)|+ $(if $guard:expr)? => $rule:expr),+$(,)?) => {{
        // why pub(crate) aaaaaaaaaaaaa
        let mut arr: InnerLangRules = [const { None }; ISOLANG_OVERVIEW_LEN];

        macro_rules! rules {
            ($dollar($r:expr),*$dollar(,)?) => {vec![$dollar({
                let rule = $r;
                rule.parse().expect(rules!(@rule))
            }),*]};
            (@$r:literal) => { concat!("cannot parse ", $r) };
            (@$r:expr) => { &format!("cannot parse {}", $r) };
        }

        preinterpret::preinterpret! { $(
            [!set! #ifguard = $(if $guard)?]
            [!set! #else = $(
                [!ignore! $guard]
                else { vec![] }
            )?]
            $(arr[Language::$Lang as usize] = Some(std::sync::Arc::new(|$l: &LanguageIdentifier, $lang: &Language| #ifguard { $rule } #else));)+
        )+ }

        return arr;
    } };
}

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

    #[allow(clippy::indexing_slicing)]
    if let Some(f) = &LANG_RULES[lang as usize] {
        rules.extend_from_slice(&f(l, &lang));
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

#[allow(unused_variables)]
pub static LANG_RULES: std::sync::LazyLock<InnerLangRules> = std::sync::LazyLock::new(|| {
    gen_langrules!(l lang:
        Ara | Arb if l.variants().len() == 0 => rules!["ar-AE", "ara-AE", "arb-AE"],
        Zho | Cmn => match l.script {
            Some(s) if s.as_str().eq_ignore_ascii_case("Hans") => {
                rules!["zh-Hans-CN", "zho-Hans-CN", "cmn-Hans-CN", "zh-Hant"]
            }
            Some(s) if s.as_str().eq_ignore_ascii_case("Hant") => {
                rules!["zh-Hant-TW", "zho-Hant-TW", "cmn-Hant-TW", "zh-Hans"]
            }
            #[allow(unused_variables)]
            Some(script) => {
                #[cfg(feature = "tracing")]
                tracing::warn!(?l, ?script, "unknown script for zho");
                vec![]
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
                    rules![format!("zh-Hans-{region}"), format!("zh-Hant-{region}")]
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
    )
});

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn isolang_overview_len() {
        assert!(Language::from_usize(ISOLANG_OVERVIEW_LEN).is_none());
        assert!(Language::from_usize(ISOLANG_OVERVIEW_LEN - 1).is_some());
    }
}
