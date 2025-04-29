#[cfg_attr(not(test), cfg(not(windows)))]
use itertools::Itertools;
use std::str::FromStr;
use unic_langid::LanguageIdentifier;

/// Obtain a list of [`LanguageIdentifier`]s the user prefers.
///
/// The behaviour of this function depends on the platform:
/// - Unix (`cfg!(unix)` except `cfg!(target_os = "macos")`): [`unix_system_want_langids`]
/// - Mac OS X (`cfg!(target_os = "macos")`): [`macos_system_want_langids`]
/// - Windows (`cfg!(windows)`): [`windows_system_want_langids`]
///
/// Even though they may not render in docs.rs, they have the same function signature to this
/// function.
///
/// Note that [`unix_system_want_langids()`] is available even on Mac OS X. In fact,
/// [`macos_system_want_langids()`] depends on that function, chaining the iterators.
pub fn system_want_langids() -> impl Iterator<Item = LanguageIdentifier> {
    #[cfg(unix)]
    #[cfg(not(target_os = "macos"))]
    {
        unix_system_want_langids()
    }
    #[cfg(target_os = "macos")]
    {
        macos_system_want_langids()
    }
    #[cfg(windows)]
    {
        windows_system_want_langids()
    }
}

/// Obtain a list of [`LanguageIdentifier`]s the user prefers, by looking up environment variables.
///
/// This function is only available on `cfg!(unix)`.
///
/// The alternatives on other platforms are:
/// - Mac OS X (`cfg!(target_os = "macos")`): [`macos_system_want_langids`]
/// - Windows (`cfg!(windows)`): [`windows_system_want_langids`]
///
/// Note that this function is available even on Mac OS X, and is used in combination.
#[cfg(unix)]
pub fn unix_system_want_langids() -> impl Iterator<Item = LanguageIdentifier> {
    ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE", "LANGUAGES"]
        .into_iter()
        .flat_map(|env| {
            std::env::var(env).ok().into_iter().flat_map(|locales| {
                locales
                    .split(':')
                    .filter_map(|locale| LanguageIdentifier::from_str(locale).ok())
                    .collect_vec()
            })
        })
}

#[cfg(target_os = "macos")]
pub fn macos_system_want_langids() -> impl Iterator<Item = LanguageIdentifier> {
    //? https://stackoverflow.com/questions/14908180/know-currently-logged-in-users-language-in-mac-via-shell-script#comment21002995_14908268
    let res = match std::process::Command::new("defaults")
        .args(["read", "NSGlobalDomain", "AppleLanguages"])
        .stdout(std::process::Stdio::piped())
        .output()
    {
        Ok(res) => res,
        #[allow(unused_variables)]
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::error!(?err, "cannot execute `defaults`");
            return Box::new(unix_system_want_langids()) as Box<dyn Iterator<Item = _>>;
        }
    };
    Box::new(macos_parse_want_langids(res.stdout).chain(unix_system_want_langids()))
}

#[cfg(target_os = "macos")]
pub fn macos_parse_want_langids(stdout: Vec<u8>) -> impl Iterator<Item = LanguageIdentifier> {
    MacSysLangidsIterator {
        positions: stdout.iter().positions(|&b| b == b',').collect_vec(),
        stdout,
        i: 0,
    }
}

#[cfg(target_os = "macos")]
pub struct MacSysLangidsIterator {
    stdout: Vec<u8>,
    positions: Vec<usize>,
    i: usize,
}

#[cfg(target_os = "macos")]
impl Iterator for MacSysLangidsIterator {
    type Item = LanguageIdentifier;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&lc) = self.positions.get(self.i) {
            self.i += 1;
            let lc =
                &self.stdout[lc + 1..*self.positions.get(self.i).unwrap_or(&self.stdout.len())];
            let lc = lc.strip_prefix(b"(").unwrap_or(lc).trim_ascii_end();
            let lc = lc.strip_suffix(b")").unwrap_or(lc).trim_ascii();
            let lc = lc
                .strip_prefix(b"\"")
                .and_then(|lc| lc.strip_suffix(b"\""))
                .unwrap_or(lc);
            match LanguageIdentifier::from_bytes(lc) {
                Ok(l) => return Some(l),
                #[allow(unused_variables)]
                Err(e) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(?lc, ?e, "invalid locale (AppleLanguages)");
                    continue;
                }
            }
        }
        None
    }
}

#[cfg(windows)]
pub fn windows_system_want_langids() -> impl Iterator<Item = LanguageIdentifier> {
    (get_system_locales().into_iter()).filter_map(|locale| {
        match LanguageIdentifier::from_str(&locale) {
            Ok(l) => return Some(l),
            Err(_) if !cfg!(feature = "tracing") => {}
            Err(err) => tracing::error!(?locale, ?err, "cannot convert to langid"),
        }
        None
    })
}

#[cfg(windows)]
fn get_system_locales() -> Vec<String> {
    let mut num_langs = 0;
    let mut buffer_size = 0;

    #[cfg_attr(not(feature = "tracing"), allow(unused_variables))]
    // SAFETY: Well we're using this API correctly :3
    if let Err(err) = unsafe {
        windows::Win32::Globalization::GetUserPreferredUILanguages(
            windows::Win32::Globalization::MUI_LANGUAGE_NAME,
            &mut num_langs,
            None,
            &mut buffer_size,
        )
    } {
        #[cfg(feature = "tracing")]
        tracing::error!(?err, "fail to get bufsize from GetUserPreferredUILanguages");
        return vec![];
    }
    let mut buffer = vec![0u16; buffer_size as usize];

    #[cfg_attr(not(feature = "tracing"), allow(unused_variables))]
    // SAFETY: Second call to retrieve the actual data
    if let Err(err) = unsafe {
        windows::Win32::Globalization::GetUserPreferredUILanguages(
            windows::Win32::Globalization::MUI_LANGUAGE_NAME,
            &mut num_langs,
            Some(windows::core::PWSTR(buffer.as_mut_ptr())),
            &mut buffer_size,
        )
    } {
        #[cfg(feature = "tracing")]
        tracing::error!(?err, "GetUserPreferredUILanguages failed");
        return vec![];
    }

    let locales = buffer
        .split(|&c| c == 0) // split on \0
        .filter(|s| !s.is_empty()) // skip last empty slice
        .filter_map(|s| {
            #[allow(unused_variables)]
            String::from_utf16(s)
                .inspect_err(|err| {
                    #[cfg(feature = "tracing")]
                    tracing::error!(?err, "cannot convert utf16");
                })
                .ok()
        })
        .collect();

    locales
}

#[cfg(not(unix))]
#[cfg(not(windows))]
compile_error!("This operating system is not supported by poly_l10n (help required!).");

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn langids() {
        println!("{:?}", system_want_langids().collect_vec());
    }
}
