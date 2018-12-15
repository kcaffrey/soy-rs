use crate::ast::{SoyFile, Template, TemplateNode};
use crate::error::{CompileError, RenderError, RenderErrorKind};
use crate::parser;
use std::collections::HashMap;

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

    pub fn render(&self, template_name: &str) -> Result<String, RenderError> {
        let mut output = String::with_capacity(8 * 1024);
        let template = self.template(template_name)?;
        // todo: handle space joining
        let mut add_space_if_text = false;
        for node in &template.body {
            match node {
                TemplateNode::RawText { value, newline } => {
                    if add_space_if_text {
                        output.push(' ');
                    }
                    output.push_str(value);
                    add_space_if_text = *newline;
                }
                TemplateNode::Statement { command, .. } => {
                    match command {
                        // TODO: implement
                        _ => {}
                    }
                    add_space_if_text = false;
                }
            }
        }
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
