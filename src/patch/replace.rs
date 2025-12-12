/// The "replace" operation replaces the value at the target location
/// with a new value.  The operation object MUST contain a "value" member
/// whose content specifies the replacement value.
///
/// The target location MUST exist for the operation to be successful.
///
/// For example:
///
/// { "op": "replace", "path": "/a/b/c", "value": 42 }
///
/// This operation is functionally identical to a "remove" operation for
/// a value, followed immediately by an "add" operation at the same
/// location with the replacement value.
fn replace()
