# eggshell-ui-shadcn

The shadcn/ui design system as an eggshell style. Nothing here runs until the
application calls `Shadcn::apply` (or `Shadcn::apply_both`).

## Fonts

With the default `fonts` feature the preset bundles **Geist**, shadcn's
default sans-serif, as one variable font, `fonts/Geist-Variable.ttf`
(`Geist[wght].ttf` from [vercel/geist-font](https://github.com/vercel/geist-font)
v1.9.2, 165 KiB). `Shadcn::apply` registers it at 400, 500, 600 and 700, so
`font_medium()`, `font_semibold()` and `font_bold()` draw real faces, and the
preset's buttons, labels and menu headings take shadcn's weights.

Geist is licensed under the SIL Open Font License 1.1; the licence is shipped
beside the font in [`fonts/OFL.txt`](fonts/OFL.txt) and applies to the font
file only. The crate's code is MIT.

- Swap the font with `Shadcn::with_fonts(Some(FontSet::variable(..)))` or
  `FontSet::faces(..)` for static files, or keep egui's fonts with
  `with_fonts(None)`.
- Leave it out with `default-features = false`: nothing is embedded, and
  weights are drawn in the regular face with a debug warning.
- egui's own `default_fonts` feature embeds Ubuntu-Light, which Geist makes a
  fallback only. Depend on egui with `default-features = false` to drop it.
