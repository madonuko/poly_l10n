# `poly_l10n`

Handle locali(s|z)ations the correct way.

There are many i18n libraries out there that doesn't do this right: language fallbacks.
Unfortunately most projects (regardless of whether or not it's open source) do not support all
languages. This library fixes this for you with [`LocaleFallbackSolver`].

Additionally this library provides helper functions for figuring out the user language preference.
See [`system_want_langids`].

## 📃 License

`GPL-3.0-or-later`

    Copyright © 2025  madonuko <mado@fyralabs.com> <madonuko@outlook.com>

    This program is free software; you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation; either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License along
    with this program; if not, write to the Free Software Foundation, Inc.,
    51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.

[`LocaleFallbackSolver`]: https://docs.rs/poly_l10n/latest/poly_l10n/struct.LocaleFallbackSolver.html
[`system_want_langids`]: https://docs.rs/poly_l10n/latest/poly_l10n/getlang/fn.system_want_langids.html
