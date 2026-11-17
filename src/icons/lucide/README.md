# Lucide icons

These SVG files are from [Lucide](https://lucide.dev), release 1.73.0, copied
from `icons/` in <https://github.com/lucide-icons/lucide> at that tag. Lucide is
under the ISC licence, in `LICENSE` beside this file.

One edit: each root `<svg>` element gains `color="#fff"`. Lucide strokes in
`currentColor`, which resolves through the `color` attribute and is otherwise
black; white lets egui's tint draw an icon in any colour.
