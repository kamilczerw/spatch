# JSON Patch test cases

This repo uses test cases from [https://github.com/json-patch/json-patch-tests](https://github.com/json-patch/json-patch-tests)
to validate if `spatch` follows the JSON Patch specification.

It should be 100% compliant with the test cases when diffing without providing json
schema with index keys set for the arrays.

The tests can be found in the `/src/diff/test_util/json_patch_tests` directory.
