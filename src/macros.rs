use unic_langid::LanguageIdentifier;

/// Create a [`unic_langid::LanguageIdentifier`] from the given string.
///
/// Note that both `langid!["en"]` and `langid!("en")` are fine and have no difference.
///
/// As far as concerned, both ISO 639-1 and 639-3 are accepted, but [`unic_langid`] considers them
/// as **DIFFERENT** language codes.
///
/// # Examples
///
/// ```
/// # use poly_l10n::{langid, unic_langid};
/// assert_eq!(
///   langid!["zho"],
///   unic_langid::LanguageIdentifier::from_bytes(b"zho").unwrap()
/// );
/// assert_eq!(langid!["en_US"], langid!["en-US"]);
/// // IMPORTANT: 639-1/2/3 all can be parsed, but they are treated as *different* IDs.
/// assert_ne!(langid!["fr"], langid!["fra"]);
/// ```
#[macro_export]
macro_rules! langid {
    ($lang:literal) => {
        $crate::macros::IntoLangIdAble::to_langid($lang).expect(concat!(
            "cannot parse language identifier langid!(\"",
            $lang,
            "\")"
        ))
    };
    ($lang:expr) => {{
        let lang = $lang;
        match $crate::macros::IntoLangIdAble::to_langid(lang) {
            Ok(id) => id,
            Err(e) => Err(e).expect(format!(
                "cannot parse language identifier langid!(\"{lang}\")"
            )),
        }
    }};
    ($($lang:tt),+$(,)?) => {[$($crate::langid!($lang)),+]}
}

/// See [`IntoLangIdAble::to_langid()`].
pub trait IntoLangIdAble {
    /// Turn `self` into `LanguageIdentifier`.
    ///
    /// This is used by the [`langid!`] macro.
    ///
    /// # Errors
    /// See [`unic_langid::LanguageIdentifierError`].
    fn to_langid(&self) -> Result<LanguageIdentifier, unic_langid::LanguageIdentifierError>;
}

impl IntoLangIdAble for str {
    fn to_langid(&self) -> Result<LanguageIdentifier, unic_langid::LanguageIdentifierError> {
        self.find('.')
            .and_then(|i| locale_str_to_langid(self, i))
            .unwrap_or_else(|| LanguageIdentifier::from_bytes(self.as_bytes()))
    }
}

#[allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]
fn locale_str_to_langid(
    locale: &str,
    i: usize,
) -> Option<Result<LanguageIdentifier, unic_langid::LanguageIdentifierError>> {
    let bs = isolang::Language::from_locale(locale)?;
    let mut count = 0;
    while (locale.as_bytes().get(count))
        .is_some_and(|b| ![b'_', b'-'].contains(b) && locale.len() > count)
    {
        count += 1;
    }
    // count is the number of characters until and excluding the `-` or the `_`
    let mut bs = if count == 2 {
        bs.to_639_1().unwrap()
    } else {
        bs.to_639_3()
    }
    .as_bytes()
    .to_owned();
    bs.extend_from_slice(&locale.as_bytes()[count + 2..i]);
    Some(LanguageIdentifier::from_bytes(&bs))
}

impl IntoLangIdAble for String {
    fn to_langid(&self) -> Result<LanguageIdentifier, unic_langid::LanguageIdentifierError> {
        self.as_str().to_langid()
    }
}
impl IntoLangIdAble for [u8] {
    fn to_langid(&self) -> Result<LanguageIdentifier, unic_langid::LanguageIdentifierError> {
        (self.iter().position(|&b| b == b'.'))
            .and_then(|i| locale_str_to_langid(core::str::from_utf8(self).ok()?, i))
            .unwrap_or_else(|| LanguageIdentifier::from_bytes(self))
    }
}
