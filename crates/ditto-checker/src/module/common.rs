use ditto_cst as cst;

pub fn extract_doc_comments<T>(token: &cst::Token<T>) -> Vec<String> {
    token
        .leading_comments
        .iter()
        .map(|comment| {
            comment
                .0
                .strip_prefix("--")
                .unwrap_or(&comment.0)
                .trim()
                .to_string()
        })
        .collect()
}
