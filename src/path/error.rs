use nom_language::error::{VerboseError, VerboseErrorKind};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PathError {
    #[error("Invalid path syntax at position {position}: {message}")]
    InvalidSyntax { position: usize, message: String },
}

impl PathError {
    pub fn invalid_syntax(position: usize, message: impl Into<String>) -> Self {
        PathError::InvalidSyntax {
            position,
            message: message.into(),
        }
    }
}

pub(super) fn convert_verbose_error(input: &str, err: VerboseError<&str>) -> PathError {
    let Some((fragment, kind)) = err.errors.last() else {
        return PathError::InvalidSyntax {
            position: 0,
            message: "invalid path syntax".to_string(),
        };
    };

    let position = input.len() - fragment.len();

    let message = match kind {
        VerboseErrorKind::Context(ctx) => ctx.to_string(),
        VerboseErrorKind::Char(c) => format!("expected '{}'", c),
        VerboseErrorKind::Nom(nom_err) => format!("parser error: {:?}", nom_err),
    };

    PathError::InvalidSyntax { position, message }
}

pub(super) const UNEXPECTED_SQ_BRACKET_MSG: &str = "unexpected '['. '[' may only appear at the start of a segment (immediately after '/'). \
                            Fix: insert a '/' before it (e.g. '/foo/[...]') or remove '['.";

pub(super) fn trailing_input_error(input: &str, rest: &str) -> PathError {
    let position = input.len().saturating_sub(rest.len());
    let ch = rest.chars().next();

    let message = match ch {
        Some('[') => UNEXPECTED_SQ_BRACKET_MSG.to_string(),
        Some(c) => format!(
            "unexpected character '{}'. Fix: remove it or check the segment syntax at this position.",
            c
        ),

        None => "unexpected end of input".to_string(),
    };

    PathError::InvalidSyntax { position, message }
}
