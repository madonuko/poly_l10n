//! Module for handling fallback rules for specific languages.
//!
//! While you may use `poly_l10n` fine without this, we believe this improves your experience
//! working with i18n in general, which is why this is enabled by default.
//!
//! This module is gated behind the feature `per_lang_default_rules`.
use isolang::Language;
use unic_langid::LanguageIdentifier;

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
        Spa if l.variants().len() == 0 => rules!["es-ES", "spa-ES", "pt-PT", "por-PT"],
        Por if l.variants().len() == 0 => rules!["pt-PT", "por-PT", "es-ES", "spa-ES"],
        Yue => match l.script {
            Some(s) if s.as_str().eq_ignore_ascii_case("Hans") => {
                rules!["yue-Hans-CN", "yue-Hant-HK", "yue-Hant-MO", "zho"]
            }
            Some(s) if s.as_str().eq_ignore_ascii_case("Hant") => {
                rules!["yue-Hant-HK", "yue-Hant-MO", "zho"]
            }
            #[allow(unused_variables)]
            Some(script) => {
                #[cfg(feature = "tracing")]
                tracing::warn!(?l, ?script, "unknown script for yue");
                vec![]
            }
            None => match l.region.as_ref().map(unic_langid::subtags::Region::as_str) {
                Some("CN" | "SG") => rules!["yue-Hans-CN", "yue-Hans-HK", "zho-Hans-CN"],
                Some("TW") => rules!["yue-Hant-TW", "yue-Hant-HK", "zho-Hant-TW"],
                Some("HK" | "MO") => rules![
                    "yue-Hant-HK",
                    "zho-Hant-HK",
                    "yue-Hant-MO",
                    "zho-Hant-MO",
                    "yue-Hant-TW",
                    "zho-Hant-TW",
                ],
                Some(region) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(region, "unknown yue region");
                    rules![format!("yue-Hans-{region}"), format!("zh-Hant-{region}")]
                }
                None => rules![ "yue-Hant-HK", "yue-Hant-MO"],
            },
        }
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
