use crate::ast::{Command, SoyFile, Template, TemplateNode};
use crate::error::{CompileError, RenderError, RenderErrorKind};
use crate::parser;
use std::collections::HashMap;
use std::io::Write;

pub struct Tofu {
    // TODO: should there be an intermediary object instead of the AST node?
    templates: HashMap<String, Template>,
}

impl Tofu {
    pub fn with_string_template(template: &str) -> Result<Tofu, CompileError> {
        let file = parser::parse(template)?;
        let mut tofu = Tofu {
            templates: HashMap::new(),
        };
        tofu.add_file(file);
        Ok(tofu)
    }

    pub fn render<W: Write>(&self, writer: W, template_name: &str) -> Result<(), RenderError> {
        let mut writer = writer;
        self.render_template(&mut writer, self.template(template_name)?)
    }

    pub fn render_to_string(&self, template_name: &str) -> Result<String, RenderError> {
        let mut output = Vec::with_capacity(8 * 1024);
        self.render(&mut output, template_name)?;
        // TODO: is it safe to use from_utf8_unchecked? probably not if we allow byte slices in input data...
        // anything that comes from a String should already be valid utf8 though
        let mut output = String::from_utf8(output)?;
        output.shrink_to_fit();
        Ok(output)
    }

    fn add_file(&mut self, file: SoyFile) {
        let namespace = file.namespace.name;
        self.templates.extend(
            file.templates
                .into_iter()
                .map(|t| (format!("{}.{}", namespace, t.name), t)),
        );
    }

    fn template(&self, name: &str) -> Result<&Template, RenderError> {
        self.templates.get(name).ok_or_else(|| RenderError {
            kind: RenderErrorKind::TemplateNotFound(name.to_owned()),
            location: Default::default(),
        })
    }
}

// Rendering
impl Tofu {
    fn render_template<W: Write>(
        &self,
        writer: &mut W,
        template: &Template,
    ) -> Result<(), RenderError> {
        // todo: handle space joining
        let mut add_space_if_text = false;
        for node in &template.body {
            match node {
                TemplateNode::RawText { value, newline } => {
                    if add_space_if_text {
                        writer.write_all(&[b' '])?;
                    }
                    writer.write_all(value.as_bytes())?;
                    add_space_if_text = *newline;
                }
                TemplateNode::Statement { command, .. } => {
                    match command {
                        Command::Literal(literal) => writer.write_all(literal.as_bytes())?,
                        Command::Msg { .. } => {}   // TODO: implement
                        Command::Print { .. } => {} // TODO: implement
                    }
                    add_space_if_text = false;
                }
                TemplateNode::Special(special) => writer.write_all(special.as_bytes())?,
            }
        }
        Ok(())
    }
}
