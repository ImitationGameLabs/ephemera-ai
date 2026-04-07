use std::fmt;

/// Error returned when content exceeds the available token budget.
pub struct TokenBudgetError {
    pub requested_tokens: usize,
    pub available_tokens: usize,
}

impl fmt::Display for TokenBudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Context budget exceeded: requested {} tokens but only {} available. \
             Use context_evict to free space before retrying.",
            self.requested_tokens, self.available_tokens
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_token_counts_and_evict_suggestion() {
        let err = TokenBudgetError { requested_tokens: 5000, available_tokens: 1200 };
        let msg = err.to_string();
        assert!(msg.contains("5000"), "should include requested tokens");
        assert!(msg.contains("1200"), "should include available tokens");
        assert!(
            msg.contains("context_evict"),
            "should suggest context_evict tool"
        );
    }
}
