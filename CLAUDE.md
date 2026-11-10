# Project rules

## Doc comments
- spatch is also a library: every doc comment on a public item must include an example.
  Prefer a compiling ```` ```rust ```` doctest.
- Private items need an example when the behavior is non-obvious (paths, indices, ordering).
  Use ```` ```text ```` or ```` ```json ````, since private-item doctests don't run.
