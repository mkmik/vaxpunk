//! Labels, as in Bazel: `//exec/mailboxes` names a component,
//! `//exec/mailboxes:basic` one of its features, and `:basic` a feature of
//! the component whose file it is written in.

use std::fmt;

/// A path segment or a feature id: `[a-z0-9][a-z0-9-]*`.
pub fn is_slug(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter()
            .all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

/// A parsed label. `path` is the component's file path under map/ without
/// `.md`, such as `exec/mailboxes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub path: String,
    pub feature: Option<String>,
}

impl Label {
    /// Parses `text` as written in the file of the component at `here`.
    /// Pass an empty `here` where `:feature` has nothing to be relative to.
    pub fn parse(text: &str, here: &str) -> Result<Label, String> {
        let (path, feature) = if let Some(rest) = text.strip_prefix("//") {
            match rest.split_once(':') {
                Some((p, f)) => (p, Some(f)),
                None => (rest, None),
            }
        } else if let Some(f) = text.strip_prefix(':') {
            if here.is_empty() {
                return Err(format!("bad label \"{text}\": here it must start with //"));
            }
            (here, Some(f))
        } else {
            return Err(format!("bad label \"{text}\": a label starts with // or :"));
        };
        if path.is_empty() || !path.split('/').all(is_slug) {
            return Err(format!(
                "bad label \"{text}\": each part of the path is lowercase letters, digits and -"
            ));
        }
        if let Some(f) = feature
            && !is_slug(f)
        {
            return Err(format!(
                "bad label \"{text}\": a feature id is lowercase letters, digits and -"
            ));
        }
        Ok(Label {
            path: path.to_string(),
            feature: feature.map(str::to_string),
        })
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "//{}", self.path)?;
        if let Some(feat) = &self.feature {
            write!(f, ":{feat}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert!(is_slug("exec"));
        assert!(is_slug("9p"));
        assert!(is_slug("attention-asts"));
        assert!(!is_slug(""));
        assert!(!is_slug("-x"));
        assert!(!is_slug("Exec"));
        assert!(!is_slug("a_b"));
    }

    #[test]
    fn parse() {
        let l = Label::parse("//exec/mailboxes", "x/y").unwrap();
        assert_eq!(l.path, "exec/mailboxes");
        assert_eq!(l.feature, None);
        let l = Label::parse("//exec/mailboxes:basic", "x/y").unwrap();
        assert_eq!(l.feature.as_deref(), Some("basic"));
        assert_eq!(l.to_string(), "//exec/mailboxes:basic");
        let l = Label::parse(":basic", "exec/mailboxes").unwrap();
        assert_eq!(l.to_string(), "//exec/mailboxes:basic");
        assert!(Label::parse(":basic", "").is_err());
        assert!(Label::parse("exec/mailboxes", "x").is_err());
        assert!(Label::parse("//exec/Mailboxes", "x").is_err());
        assert!(Label::parse("//exec//mailboxes", "x").is_err());
        assert!(Label::parse("//exec/mailboxes:", "x").is_err());
        assert!(Label::parse("//", "x").is_err());
    }
}
