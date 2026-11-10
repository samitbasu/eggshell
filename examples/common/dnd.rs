//! What the drag-and-drop examples share.

/// Moves the item at `from` to `to`, the others keeping their order.
pub fn move_to<T>(items: &mut [T], from: usize, to: usize) {
    if from < to {
        items[from..=to].rotate_left(1);
    } else {
        items[to..=from].rotate_right(1);
    }
}

#[cfg(test)]
mod tests {
    use super::move_to;

    #[test]
    fn moves_forward_and_back() {
        let mut items = [0, 1, 2, 3, 4];
        move_to(&mut items, 1, 3);
        assert_eq!(items, [0, 2, 3, 1, 4]);
        move_to(&mut items, 3, 0);
        assert_eq!(items, [1, 0, 2, 3, 4]);
        move_to(&mut items, 2, 2);
        assert_eq!(items, [1, 0, 2, 3, 4]);
    }
}
