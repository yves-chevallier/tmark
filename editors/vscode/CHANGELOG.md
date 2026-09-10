# Changelog

## Unreleased

- Language server client: starts `tmark-lsp` (bundled, on the PATH, or
  `tmark.serverPath`) for `tmark` and Markdown files; diagnostics, outline,
  folding, formatting. `.tm` joins the recognised extensions.

## 0.1.0

- Initial release: TextMate grammar for TMark (draft 3 of `spec/tmark.md`),
  injected into every Markdown document and available as the `tmark` language.
