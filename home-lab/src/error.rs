use anyhow::Error;

/// Extract all the nested error messages
pub fn error_messages(error: &Error) -> String {
    let mut messages = vec![error.to_string()];
    let mut source = error.source();
    while let Some(err) = source {
        if messages.len() == 1 {
            messages.push("Caused by:".to_string());
        }
        messages.push(format!("- {}", err));
        source = err.source();
    }
    messages.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let a = Error::msg("one");
        assert_eq!(error_messages(&a), "one");

        let b = a.context("two");
        let c = b.context("three");

        assert_eq!(error_messages(&c), "three\nCaused by:\n- two\n- one");
    }
}
