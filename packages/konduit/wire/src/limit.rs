//! Resources are limited.

use problem_details::ProblemDetail;

#[derive(Debug, Clone, ProblemDetail)]
pub enum LimitError {
    /// Resource limit reached. Cannot carryout additional action.
    /// User must wait.
    #[problem(slug = "limit-reached", title = "Limit Reached", http_status = 400)]
    Reached,
}
