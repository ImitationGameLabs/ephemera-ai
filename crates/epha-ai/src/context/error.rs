use std::fmt;

/// Error returned when content exceeds the available token budget.
pub struct TokenBudgetError {
    pub requested_tokens: usize,
    pub available_tokens: usize,
}

/// Error returned when pinning a memory would exceed the pinned token budget.
#[derive(Debug)]
pub struct PinnedTokenBudgetError {
    pub over_tokens: usize,
    pub used_tokens: usize,
    pub max_tokens: usize,
}

impl fmt::Display for PinnedTokenBudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Pinned memory budget exceeded: over by {} tokens (using {}/{}). Unpin some memories first.",
            self.over_tokens, self.used_tokens, self.max_tokens
        )
    }
}

/// Error returned when pinning a memory fails.
#[derive(Debug)]
pub enum PinError {
    AlreadyPinned,
    ApiError(String),
    BudgetExceeded(PinnedTokenBudgetError),
}

impl fmt::Display for PinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PinError::AlreadyPinned => write!(f, "Memory is already pinned"),
            PinError::ApiError(inner) => write!(f, "Failed to pin memory: {inner}"),
            PinError::BudgetExceeded(e) => fmt::Display::fmt(e, f),
        }
    }
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
