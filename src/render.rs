use crate::proto;
use askama::Template;

#[derive(Template)]
#[template(path = "template.md")]
pub struct Page<'a> {
    services: Vec<proto::Service<'a>>,
}

impl<'a> From<Vec<proto::Service<'a>>> for Page<'a> {
    fn from(services: Vec<proto::Service<'a>>) -> Self {
        Self { services }
    }
}

mod filters {
    /// Split lines in `s` and prepend each line with `//` and join back.
    #[allow(clippy::unnecessary_wraps)]
    pub fn render_multiline_comment<T: std::fmt::Display>(s: T) -> askama::Result<String> {
        Ok(s.to_string()
            .lines()
            .map(|s| {
                let mut s = s.to_string();
                s.insert_str(0, "//");
                s
            })
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::filters::render_multiline_comment;

    #[test]
    fn render_multiline_comments() {
        assert_eq!(
            render_multiline_comment("foo\nbar").unwrap(),
            "//foo\n//bar"
        );
    }
}
