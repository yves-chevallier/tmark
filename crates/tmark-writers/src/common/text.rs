//! Shared Unicode tables (dashes, quotes, super/subscript runs), the key
//! label lookup, and the slug rules writers share (writers-and-passes.md
//! §1 `common/text.rs`, fragment-contracts.md open question 2).

/// `–`, `‒`, `—`, `―` → `--` / `---` (`escaper.py:232`).
pub const DASHES: &[(char, &str)] = &[
    ('\u{2013}', "--"),
    ('\u{2012}', "--"),
    ('\u{2014}', "---"),
    ('\u{2015}', "---"),
];

/// Smart quotes and the ellipsis to their ASCII TeX spellings
/// (`escaper.py:240`).
pub const PUNCTUATION: &[(char, &str)] = &[
    ('\u{2019}', "'"),
    ('\u{2018}', "`"),
    ('\u{201A}', ","),
    ('\u{201B}', "'"),
    ('\u{201C}', "``"),
    ('\u{201D}', "''"),
    ('\u{201E}', ",,"),
    ('\u{201F}', "''"),
    ('\u{2026}', "..."),
];

/// Unicode superscript characters and their base (`escaper.py:124`).
pub const SUPERSCRIPTS: &[(char, &str)] = &[
    ('⁰', "0"),
    ('¹', "1"),
    ('²', "2"),
    ('³', "3"),
    ('⁴', "4"),
    ('⁵', "5"),
    ('⁶', "6"),
    ('⁷', "7"),
    ('⁸', "8"),
    ('⁹', "9"),
    ('⁺', "+"),
    ('⁻', "-"),
    ('⁼', "="),
    ('⁽', "("),
    ('⁾', ")"),
    ('ⁿ', "n"),
    ('ⁱ', "i"),
    ('ᵃ', "a"),
    ('ᵇ', "b"),
    ('ᶜ', "c"),
    ('ᵈ', "d"),
    ('ᵉ', "e"),
    ('ᶠ', "f"),
    ('ᵍ', "g"),
    ('ʰ', "h"),
    ('ᶦ', "i"),
    ('ʲ', "j"),
    ('ᵏ', "k"),
    ('ˡ', "l"),
    ('ᵐ', "m"),
    ('ᶰ', "n"),
    ('ᵒ', "o"),
    ('ᵖ', "p"),
    ('ʳ', "r"),
    ('ˢ', "s"),
    ('ᵗ', "t"),
    ('ᵘ', "u"),
    ('ᵛ', "v"),
    ('ʷ', "w"),
    ('ˣ', "x"),
    ('ʸ', "y"),
    ('ᶻ', "z"),
    ('ᴬ', "A"),
    ('ᴮ', "B"),
    ('ᴰ', "D"),
    ('ᴱ', "E"),
    ('ᴳ', "G"),
    ('ᴴ', "H"),
    ('ᴵ', "I"),
    ('ᴶ', "J"),
    ('ᴷ', "K"),
    ('ᴸ', "L"),
    ('ᴹ', "M"),
    ('ᴺ', "N"),
    ('ᴼ', "O"),
    ('ᴾ', "P"),
    ('ᴿ', "R"),
    ('ᵀ', "T"),
    ('ᵁ', "U"),
    ('ⱽ', "V"),
    ('ᵂ', "W"),
];

/// Unicode subscript characters and their base (`escaper.py:190`). The
/// Greek entries map to bare macros as the legacy table did
/// (`escaper.py:223`; reproduced, see the handoff).
pub const SUBSCRIPTS: &[(char, &str)] = &[
    ('₀', "0"),
    ('₁', "1"),
    ('₂', "2"),
    ('₃', "3"),
    ('₄', "4"),
    ('₅', "5"),
    ('₆', "6"),
    ('₇', "7"),
    ('₈', "8"),
    ('₉', "9"),
    ('₊', "+"),
    ('₋', "-"),
    ('₌', "="),
    ('₍', "("),
    ('₎', ")"),
    ('ₐ', "a"),
    ('ₑ', "e"),
    ('ₒ', "o"),
    ('ₔ', "ə"),
    ('ₓ', "x"),
    ('ₕ', "h"),
    ('ₖ', "k"),
    ('ₗ', "l"),
    ('ₘ', "m"),
    ('ₙ', "n"),
    ('ₚ', "p"),
    ('ₛ', "s"),
    ('ₜ', "t"),
    ('ᵢ', "i"),
    ('ᵣ', "r"),
    ('ᵤ', "u"),
    ('ᵥ', "v"),
    ('ᵦ', "\\beta"),
    ('ᵧ', "\\gamma"),
    ('ᵨ', "\\rho"),
    ('ᵩ', "\\phi"),
    ('ᵪ', "\\chi"),
];

pub fn lookup(table: &[(char, &'static str)], c: char) -> Option<&'static str> {
    table.iter().find(|(k, _)| *k == c).map(|(_, v)| *v)
}

/// The label of a key: the registry row (`tmark_ir::registry::KEY_LABELS`,
/// case-insensitive), else the key upper-cased (`keystroke.tex`:
/// `key|upper`).
pub fn key_label(key: &str) -> String {
    tmark_ir::registry::key_label(key.trim())
        .map(|k| k.label.to_string())
        .unwrap_or_else(|| key.trim().to_uppercase())
}

/// ASCII fold of the Latin letters with diacritics `unidecode` would
/// transliterate; other characters are dropped by the slug rules.
pub fn ascii_fold(c: char) -> Option<&'static str> {
    Some(match c {
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'Ā' | 'Ă' | 'Ą' => "A",
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'Æ' => "AE",
        'æ' => "ae",
        'Ç' | 'Ć' | 'Č' | 'Ĉ' | 'Ċ' => "C",
        'ç' | 'ć' | 'č' | 'ĉ' | 'ċ' => "c",
        'Ď' | 'Đ' | 'Ð' => "D",
        'ď' | 'đ' | 'ð' => "d",
        'È' | 'É' | 'Ê' | 'Ë' | 'Ē' | 'Ĕ' | 'Ė' | 'Ę' | 'Ě' => "E",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'Ğ' | 'Ĝ' | 'Ġ' | 'Ģ' => "G",
        'ğ' | 'ĝ' | 'ġ' | 'ģ' => "g",
        'Ĥ' | 'Ħ' => "H",
        'ĥ' | 'ħ' => "h",
        'Ì' | 'Í' | 'Î' | 'Ï' | 'Ī' | 'Ĭ' | 'Į' | 'İ' => "I",
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
        'Ĵ' => "J",
        'ĵ' => "j",
        'Ķ' => "K",
        'ķ' => "k",
        'Ĺ' | 'Ļ' | 'Ľ' | 'Ŀ' | 'Ł' => "L",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        'Ñ' | 'Ń' | 'Ņ' | 'Ň' => "N",
        'ñ' | 'ń' | 'ņ' | 'ň' => "n",
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' | 'Ō' | 'Ŏ' | 'Ő' => "O",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
        'Œ' => "OE",
        'œ' => "oe",
        'Ŕ' | 'Ŗ' | 'Ř' => "R",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'Ś' | 'Ŝ' | 'Ş' | 'Š' | 'Ș' => "S",
        'ś' | 'ŝ' | 'ş' | 'š' | 'ș' => "s",
        'ß' => "ss",
        'Ţ' | 'Ť' | 'Ŧ' | 'Ț' => "T",
        'ţ' | 'ť' | 'ŧ' | 'ț' => "t",
        'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ū' | 'Ŭ' | 'Ů' | 'Ű' | 'Ų' => "U",
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        'Ŵ' => "W",
        'ŵ' => "w",
        'Ý' | 'Ŷ' | 'Ÿ' => "Y",
        'ý' | 'ÿ' | 'ŷ' => "y",
        'Ź' | 'Ż' | 'Ž' => "Z",
        'ź' | 'ż' | 'ž' => "z",
        'Þ' => "Th",
        'þ' => "th",
        _ => return None,
    })
}

/// `python-slugify` with its defaults: ASCII fold, lowercase, runs of
/// non-alphanumerics become one `-`, none at the ends. The heading slug
/// rule of writers-and-passes.md §2 "Headings" (open question b: decided
/// on the plain text of the heading).
pub fn slugify(text: &str) -> String {
    slug_with(text, "-", true)
}

/// `0.45` for 0.45, `1` for 1.0: a fraction with at most four decimals
/// and no trailing zeros (progress bar values).
pub fn trim_float(value: f64) -> String {
    let text = format!("{value:.4}");
    let text = text.trim_end_matches('0');
    text.trim_end_matches('.').to_string()
}

/// The acronym key of `\tsacr{key}` (`context.py:52-98`:
/// `slugify(term, separator="", lowercase=False)`); a caller adds the
/// `2`, `3`, … collision suffixes.
pub fn acronym_key(term: &str) -> String {
    let key = slug_with(term, "", false);
    if key.is_empty() {
        "acronym".to_string()
    } else {
        key
    }
}

fn slug_with(text: &str, separator: &str, lowercase: bool) -> String {
    let mut folded = String::with_capacity(text.len());
    for c in text.chars() {
        match ascii_fold(c) {
            Some(s) => folded.push_str(s),
            None => folded.push(c),
        }
    }
    let mut out = String::with_capacity(folded.len());
    let mut pending = false;
    for c in folded.chars() {
        if c.is_ascii_alphanumeric() {
            if pending && !out.is_empty() {
                out.push_str(separator);
            }
            pending = false;
            if lowercase {
                out.push(c.to_ascii_lowercase());
            } else {
                out.push(c);
            }
        } else {
            pending = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slugify("Boot sequence"), "boot-sequence");
        assert_eq!(
            slugify("Core Markdown (Standard “Vanilla”)"),
            "core-markdown-standard-vanilla"
        );
        assert_eq!(slugify("  Éléphant à l'œuvre  "), "elephant-a-l-oeuvre");
        assert_eq!(slugify("日本語"), "");
        assert_eq!(acronym_key("HTML"), "HTML");
        assert_eq!(acronym_key("Wi-Fi 6"), "WiFi6");
        assert_eq!(acronym_key("---"), "acronym");
    }

    #[test]
    fn keys() {
        assert_eq!(key_label("ctrl"), "Ctrl");
        assert_eq!(key_label("Control"), "Ctrl");
        assert_eq!(key_label("s"), "S");
        assert_eq!(key_label("arrow-up"), "↑");
    }
}
