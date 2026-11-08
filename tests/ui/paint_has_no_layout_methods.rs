use bullseye::tw;

/// ```html
/// <div class="hover:p-4"></div>
/// ```
fn main() {
    let _ = tw().hover(|paint| paint.p(4.0));
}
