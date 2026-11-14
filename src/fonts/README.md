# Bundled fonts

`DejaVuSans-symbols.ttf` is DejaVu Sans 2.37 cut down to the symbols that
key chips and menu shortcuts write and that neither egui's faces nor Geist
carry: ⌘ ⇧ ⌥ ⌃ ⌫ ↵ ⇥ ← ↑ → ↓ ⏎. It is registered as the last fallback of the
proportional family (`src/symbols.rs`), so it never replaces a glyph the
text face has. Its licence is `DejaVuSans-LICENSE.txt`, and is embedded in
the font's name table.

Regenerate it from DejaVu Sans 2.37 with fontTools:

```sh
python3 -m fontTools.subset DejaVuSans.ttf \
  --unicodes=U+2318,U+21E7,U+2325,U+2303,U+232B,U+21B5,U+21E5,U+2190-2193,U+23CE \
  --no-hinting --no-glyph-names --name-IDs=0,1,2,3,4,5,6,13,14 --notdef-outline \
  --drop-tables+=GDEF,GPOS,GSUB,MATH,gasp,DSIG \
  --output-file=DejaVuSans-symbols.ttf
```
