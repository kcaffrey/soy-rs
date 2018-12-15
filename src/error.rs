use std::error::Error;
use std::fmt;
use std::io;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub struct TemplateLocation {
    pub filename: Option<String>,
    pub template_name: Option<String>,
    pub line_number: usize,
    pub column: usize,
    pub snippet: Option<String>,
}

#[derive(Debug)]
pub struct RenderError {
    pub kind: RenderErrorKind,
    pub location: Option<TemplateLocation>,
}

#[derive(Debug)]
pub enum RenderErrorKind {
    IoError(io::Error),
    Utf8Error(FromUtf8Error),
    TemplateNotFound(String),
    // TODO: more error kinds
}

#[derive(Debug)]
pub struct CompileError {
    pub kind: CompileErrorKind,
    pub location: Option<TemplateLocation>,
    pub cause: Option<Box<std::error::Error>>,
}

#[derive(Debug)]
pub enum CompileErrorKind {
    Parse,
    UndeclaredParameter(String),
    // TODO: more error kinds
}

impl Error for RenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl Error for CompileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl From<io::Error> for RenderError {
    fn from(from: io::Error) -> Self {
        RenderError {
            kind: RenderErrorKind::IoError(from),
            location: None,
        }
    }
}

impl From<FromUtf8Error> for RenderError {
    fn from(from: FromUtf8Error) -> Self {
        RenderError {
            kind: RenderErrorKind::Utf8Error(from),
            location: None,
        }
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::RenderErrorKind::*;
        match &self.kind {
            TemplateNotFound(t) => write!(f, "Template not found: {}", t)?,
            IoError(e) => write!(f, "IO Error: {}", e)?,
            Utf8Error(e) => write!(f, "UTF8 Encoding Error: {}", e)?,
        }
        if let Some(location) = &self.location {
            write!(f, "\n{}", location)?;
        }
        Ok(())
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::CompileErrorKind::*;
        match (&self.kind, &self.cause) {
            (Parse, Some(cause)) => write!(f, "{}", cause)?,
            (Parse, _) => write!(f, "Parse error")?,
            (UndeclaredParameter(param), _) => {
                write!(f, "Usage of undeclared parameter: {}", param)?
            }
        }
        if let Some(location) = &self.location {
            write!(f, "\n{}", location)?;
        }
        Ok(())
    }
}

impl fmt::Display for TemplateLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.filename {
            Some(name) => write!(
                f,
                "{}: line {}, column {}",
                name, self.line_number, self.column
            )?,
            None => write!(f, "Line {}, column {}", self.line_number, self.column)?,
        }
        if let Some(name) = &self.template_name {
            write!(f, ", in {}", name)?;
        }
        writeln!(f)?;
        if let Some(snippet) = &self.snippet {
            writeln!(f, "{}", snippet)?;
        }
        Ok(())
    }
}
