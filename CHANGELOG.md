# Changelog

## 0.2.2

Additive release: nothing in the 0.2.1 public API changed.

### Added

- **CSS font-size helpers.** `size` has always meant ascent minus descent in pixels, while a
  browser's `font-size` is the em size, so the same number drew at a different scale.
  `LoadFont::css_to_size(css_px)` converts a CSS size to `size` (and
  `LoadFont::size_to_css` goes back), and `TextOptions::css_size(css_px)` sets the size from a
  CSS value, so text can be drawn and measured at the size a browser would use.

### Fixed

- **Script-dependent glyph choice for neutral characters.** Spaces, punctuation, digits and
  marks now take the script of the text before them (like browser run itemization) instead
  of the script of the bidi run they land in. In a font whose `locl` / `calt` rules depend on
  the script (for example a narrower Arabic space), a space after Arabic text now gets the
  Arabic form even when it sits in a left-to-right run, as in mixed Arabic / Latin lines.
- Widths now match a browser's laid-out text (DOM) within 1% for the tested fonts, which the
  new `tests/browser_widths.rs` checks against fixed browser measurements.
