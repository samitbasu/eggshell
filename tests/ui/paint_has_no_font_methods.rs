use bullseye::tw;

/// ```html
/// <div class="group-hover:italic"></div>
/// ```
fn main() {
    let _ = tw().group_hover(|paint| paint.italic());
}
