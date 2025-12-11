# Patch Test Cases

The tool should be very close to the JsonPatch spec (RFC 6902). There is a repo with
a lot of test cases for JsonPatch implementations:
https://github.com/json-patch/json-patch-tests

It won't immediately work with our tool, because it uses JsonPointer (RFC 6901) paths,
our tool uses custom format - which is based on JsonPointer, but it also supports
finding elements in arrays by property values like `/items[id=12345]/name`.

The test cases can be adapted to work with our tool by converting the paths to our
format.

## Testing the diff

We can also use the test cases to verify the diff functionality, by diffing the document
with the expected result and comparing the generated patch with the original patch.
