//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt;

/// Helper structure to format nested structures using the debug formatter.
pub(crate) struct Fmt<F>(pub(crate) F);
impl<F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result> fmt::Debug for Fmt<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f)
    }
}

/// Helper structure to remove double-quotes around stringified keys in display outputs.
pub(crate) struct ViaDisplayNoAlloc<'a, T: fmt::Display + ?Sized>(pub(crate) &'a T);
impl<'a, T: fmt::Display + ?Sized> fmt::Debug for ViaDisplayNoAlloc<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0, f)
    }
}

/// Helper structure to remove double-quotes around stringified keys in display outputs.
pub(crate) struct ViaDisplay<T: fmt::Display + ?Sized>(pub(crate) T);
impl<T: fmt::Display + ?Sized> fmt::Debug for ViaDisplay<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
