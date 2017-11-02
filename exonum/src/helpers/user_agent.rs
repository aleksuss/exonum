//! Information about current node including Exonum, Rust and OS versions.

static USER_AGENT: &str = include_str!(concat!(env!("OUT_DIR"), "/user_agent"));

/// Returns "user agent" string containing information about Exonum, Rust and OS versions.
///
/// # Examples
///
/// ```
/// use exonum::helpers::user_agent;
///
/// let user_agent = user_agent::get();
/// println!("{}", user_agent);
/// ```
pub fn get() -> String {
    // TODO Add a functionality to find out an user agent from host
    format!("{}/{}", USER_AGENT, "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Checks that user agent string contains three nonempty components.
    #[test]
    fn components() {
        let user_agent = get();
        let components: Vec<_> = user_agent.split('/').collect();
        assert_eq!(components.len(), 3);

        for val in components {
            assert!(!val.is_empty());
        }
    }
}
