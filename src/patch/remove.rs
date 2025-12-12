/// The "remove" operation removes the value at the target location.
///
/// The target location MUST exist for the operation to be successful.
///
/// For example:
///
/// { "op": "remove", "path": "/a/b/c" }
///
/// If removing an element from an array, any elements above the
/// specified index are shifted one position to the left.
fn remove()
