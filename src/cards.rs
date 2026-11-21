// SPDX-License-Identifier: MIT
//! Which card images are SuperGrafx cards. **Not from the reference**: supergrafx-python is
//! handed a card by its caller. A host that takes any HuCard image (a browser page) needs to
//! know which machine to build, and a HuCard has no header that says.
//!
//! Five cards were sold for the SuperGrafx alone; they are named here by the SHA-1 of the image
//! (without a copier header), the user's library as measured, nothing of the images themselves.
//! Dumps of them carry the extension `.sgx`. The two PC Engine cards with extra SuperGrafx
//! graphics (Darius Plus, Darius Alpha) are not listed: they are PC Engine cards, run on the
//! plain board, and nothing here was measured on them.

use pcengine_rust::cards::strip_copier_header;
use sha1::{Digest, Sha1};

/// (SHA-1 of the image, title) of the five SuperGrafx cards.
pub const SUPERGRAFX_CARDS: [(&str, &str); 5] = [
    (
        "66873b0e0cc699aa75daf06a3f74c0508037e605",
        "1941 - Counter Attack",
    ),
    ("d876248ee1b80ad11633fd4be8aa21a187b52193", "Aldynes"),
    ("edc8f7d2233c92ebd309ac804527c461851a4b63", "Battle Ace"),
    ("cab9e4a1a8b6da07cc2c5cc35949928199a417ee", "Daimakaimura"),
    (
        "5ed16565753c902e14da9a0c5361ef01fa74463b",
        "Madou King Granzort",
    ),
];

/// The title of the SuperGrafx card this image is, if it is one of the five.
pub fn supergrafx_title(image: &[u8]) -> Option<&'static str> {
    let digest: String = Sha1::digest(strip_copier_header(image))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    SUPERGRAFX_CARDS
        .iter()
        .find(|(sha1, _)| *sha1 == digest)
        .map(|(_, title)| *title)
}

/// Should a host build a SuperGrafx for this card? Yes for one of the five known images, and
/// for a file the user names `.sgx` (another dump or a translation of one, a homebrew card).
pub fn is_supergrafx_card(image: &[u8], file_name: Option<&str>) -> bool {
    let named = file_name.is_some_and(|name| name.to_ascii_lowercase().ends_with(".sgx"));
    named || supergrafx_title(image).is_some()
}
