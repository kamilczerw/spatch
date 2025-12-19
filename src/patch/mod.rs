mod add;
mod copy;
mod error;
mod move_op;
mod remove;
mod replace;
mod test;

pub use add::add;
pub use copy::copy;
pub use move_op::move_op;
pub use remove::remove;
pub use replace::replace;
pub use test::test;

#[cfg(test)]
mod tests {

    #[test]
    fn applying_patch_with_failing_test_should_not_apply_any_changes() {
        // TODO: Implement this test based on RFC 6902 Section 5
        // https://datatracker.ietf.org/doc/html/rfc6902#section-5
    }
}
