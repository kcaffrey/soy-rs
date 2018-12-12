use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct RenderError {
    pub kind: RenderErrorKind,
    pub filename: Option<String>,
    pub template_name: Option<String>,
    pub line_number: Option<u32>,
    pub column: Option<u32>,
    pub snippet: Option<String>,
    pub cause: Option<&'static (dyn Error + Sync + Send)>,
}

#[derive(Debug)]
pub enum RenderErrorKind {
    Unknown,
    TemplateNotFound(String),
    // TODO: more error kinds
}

impl Default for RenderError {
    fn default() -> Self {
        RenderError {
            kind: RenderErrorKind::Unknown,
            filename: None,
            template_name: None,
            line_number: None,
            column: None,
            snippet: None,
            cause: None,
        }
    }
}

impl Error for RenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self.cause {
            None => None,
            Some(c) => Some(&*c),
        }
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: implement
        write!(f, "{}", self.kind)
    }
}

impl fmt::Display for RenderErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::RenderErrorKind::*;
        match self {
            Unknown => write!(f, "Unknown render error"),
            TemplateNotFound(t) => write!(f, "template not found: {}", t),
        }
    }
}
