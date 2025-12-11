//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::pallas::ed25519;
use std::{cmp, fmt, str::FromStr};

/// An EdDSA signature on Curve25519.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(transparent)]
pub struct Signature(pub ed25519::Signature);

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl PartialOrd for Signature {
    fn partial_cmp(&self, rhs: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(rhs))
    }
}

impl Ord for Signature {
    fn cmp(&self, rhs: &Self) -> cmp::Ordering {
        let lhs = self.as_ref();
        let rhs = rhs.as_ref();
        lhs.cmp(rhs)
    }
}
// ------------------------------------------------------------------ Inspecting

impl Signature {
    pub const SIZE: usize = ed25519::Signature::SIZE;
}

// ----------------------------------------------------------- Converting (from)

impl From<[u8; 64]> for Signature {
    fn from(value: [u8; 64]) -> Self {
        Self(ed25519::Signature::from(value))
    }
}

impl FromStr for Signature {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        Ok(Signature(ed25519::Signature::from_str(s)?))
    }
}

impl From<ed25519::Signature> for Signature {
    fn from(sig: ed25519::Signature) -> Self {
        Self(sig)
    }
}

// ------------------------------------------------------------- Converting (to)

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<Signature> for [u8; 64] {
    fn from(sig: Signature) -> Self {
        // Only way to 'leak' bytes is via slice?!
        <[u8; 64]>::try_from(sig.0.as_ref())
            .unwrap_or_else(|e| unreachable!("couldn't convert signature; not 64 bytes: {e:?}"))
    }
}

impl From<Signature> for ed25519::Signature {
    fn from(sig: Signature) -> Self {
        sig.0
    }
}

impl<'a> From<&'a Signature> for &'a ed25519::Signature {
    fn from(sig: &'a Signature) -> Self {
        &sig.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_fixed_array() {
        let inner = [0; 64];
        let sig = Signature::from(inner.clone());
        let re_inner = <[u8; 64]>::from(sig);
        assert_eq!(inner, re_inner, "Failed roundtrip");
    }
}
