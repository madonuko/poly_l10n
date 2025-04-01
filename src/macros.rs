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
        LanguageIdentifier::from_bytes(self.as_bytes())
    }
}
impl IntoLangIdAble for String {
    fn to_langid(&self) -> Result<LanguageIdentifier, unic_langid::LanguageIdentifierError> {
        LanguageIdentifier::from_bytes(self.as_bytes())
    }
}
impl IntoLangIdAble for [u8] {
    fn to_langid(&self) -> Result<LanguageIdentifier, unic_langid::LanguageIdentifierError> {
        LanguageIdentifier::from_bytes(self)
    }
}
