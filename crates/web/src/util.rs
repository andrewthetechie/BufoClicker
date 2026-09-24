//! Small view helpers.

use leptos::prelude::*;

/// An empty `AnyView`, for `match` arms that render nothing.
#[allow(clippy::unit_arg)]
pub fn empty_view() -> AnyView {
    ().into_any()
}
