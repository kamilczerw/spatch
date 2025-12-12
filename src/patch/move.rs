/// The "move" operation removes the value at a specified location and
/// adds it to the target location.
///
/// The operation object MUST contain a "from" member, which is a string
/// containing a JSON Pointer value that references the location in the
/// target document to move the value from.
///
/// The "from" location MUST exist for the operation to be successful.
///
/// For example:
///
/// { "op": "move", "from": "/a/b/c", "path": "/a/b/d" }
///
/// This operation is functionally identical to a "remove" operation on
/// the "from" location, followed immediately by an "add" operation at
/// the target location with the value that was just removed.
///
/// The "from" location MUST NOT be a proper prefix of the "path"
/// location; i.e., a location cannot be moved into one of its children.
fn move() {
    // Implementation goes here
}
