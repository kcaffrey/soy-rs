use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;

pub mod ast;
pub use self::ast::*;

#[cfg(test)]
mod tests;

#[derive(Parser)]
#[grammar = "parser/soy.pest"]
struct SoyParser;

pub fn parse(input: &str) -> Result<SoyFile, Error<Rule>> {
    Ok(parse_soyfile(
        SoyParser::parse(Rule::soy_file, input)?.next().unwrap(),
    ))
}

fn parse_soyfile(pair: Pair<Rule>) -> SoyFile {
    let mut delpackage = None;
    let mut namespace = None;
    let mut aliases = vec![];
    let mut templates = vec![];
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::namespace => namespace = Some(parse_namespace(p)),
            Rule::alias => aliases.push(parse_alias(p)),
            Rule::template => templates.push(parse_template(p)),
            Rule::delpackage => {
                delpackage = Some(p.into_inner().next().unwrap().as_str().to_owned())
            }
            Rule::EOI => {}
            unrecognized => unreachable!("parse soyfile: {:?}", unrecognized),
        }
    }
    SoyFile {
        delpackage,
        namespace: namespace.expect("expecting namespace"),
        aliases,
        templates,
    }
}

fn parse_namespace(pair: Pair<Rule>) -> Namespace {
    let mut name = None;
    let mut attributes = HashMap::new();
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::global_name => name = Some(p.as_str().to_owned()),
            Rule::attributes => {
                for attr in p.into_inner() {
                    let mut attr = attr.into_inner();
                    let name = attr.next().unwrap().as_str().to_owned();
                    let value = parse_quoted_string(attr.next().unwrap());
                    attributes.insert(name, value);
                }
            }
            unrecognized => unreachable!("parse namespace: {:?}", unrecognized),
        }
    }
    Namespace {
        name: name.expect("expecting name"),
        attributes,
    }
}

fn parse_alias(pair: Pair<Rule>) -> Alias {
    let mut from = None;
    let mut to = None;
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::global_name => from = Some(p.as_str().to_owned()),
            Rule::alias_as => to = Some(p.into_inner().next().unwrap().as_str().to_owned()),
            unrecognized => unreachable!("parse alias: {:?}", unrecognized),
        }
    }
    Alias {
        from: from.expect("expecting name"),
        to,
    }
}

fn parse_template(pair: Pair<Rule>) -> Template {
    let mut soydoc_params = vec![];
    let mut body = None;
    let mut name = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::soydoc => soydoc_params = p.into_inner().map(parse_soydoc_param).collect(),
            Rule::template_name => {
                let p = p.into_inner().next().unwrap();
                name = Some(match p.as_rule() {
                    Rule::partial_name => {
                        TemplateName::Partial(p.into_inner().next().unwrap().as_str().to_owned())
                    }
                    Rule::global_name => TemplateName::Global(p.as_str().to_owned()),
                    unrecognized => unreachable!("parse template name: {:?}", unrecognized),
                });
            }
            Rule::template_block => body = Some(parse_template_block(p)),
            _ => {}
        }
    }

    Template {
        name: name.expect("expecting name"),
        body: body.expect("expecting template body"),
        soydoc_params,
    }
}

fn parse_soydoc_param(pair: Pair<Rule>) -> SoydocParam {
    let mut name = None;
    let mut required = None;
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::name => name = Some(p.as_str().to_owned()),
            Rule::soydoc_required => required = Some(true),
            Rule::soydoc_optional => required = Some(false),
            unrecognized => unreachable!("parse soydoc param: {:?}", unrecognized),
        }
    }
    SoydocParam {
        name: name.expect("expecting name"),
        required: required.expect("expecting required"),
    }
}

fn parse_template_block(pair: Pair<Rule>) -> TemplateBlock {
    pair.into_inner()
        .flat_map(|p| {
            let mut has_linebreak = false;
            let mut command = None;
            let mut raw_text = None;
            for p in p.into_inner() {
                match p.as_rule() {
                    Rule::linebreak | Rule::inner_comment | Rule::multiline_comment => {
                        has_linebreak = true
                    }
                    Rule::statement => {
                        command = Some(parse_command(p.into_inner().next().unwrap()))
                    }
                    Rule::raw_text => raw_text = Some(p.as_str().to_owned()),
                    unrecognized => unreachable!("parse template block: {:?}", unrecognized),
                };
            }
            if let Some(command) = command {
                Some(TemplateNode::Statement {
                    command,
                    has_linebreak,
                })
            } else if let Some(raw_text) = raw_text {
                Some(TemplateNode::RawText {
                    value: raw_text,
                    has_linebreak,
                })
            } else {
                None
            }
        })
        .collect()
}

fn parse_expression(pair: Pair<Rule>) -> Expression {
    match pair.as_rule() {
        Rule::expression => parse_expression(pair.into_inner().next().unwrap()),
        Rule::null => Expression::Null,
        Rule::boolean => Expression::Boolean(pair.as_str().parse().unwrap()),
        Rule::integer => Expression::Integer(pair.as_str().parse().unwrap()),
        Rule::float => Expression::Float(pair.as_str().parse().unwrap()),
        Rule::string => Expression::String(parse_quoted_string(pair.into_inner().next().unwrap())),
        Rule::operation => {
            let mut p = pair.into_inner();
            let lhs = parse_expression(p.next().unwrap());
            let mut ops = Vec::new();
            while let Some(op) = p.next() {
                let op = parse_binary_operator(&op);
                let rhs = parse_expression(p.next().unwrap());
                ops.push((op, rhs));
            }
            build_binary_operation(lhs, ops)
        }
        Rule::ternary_operation => {
            let mut p = pair.into_inner();
            Expression::TernaryOperation {
                condition: Box::new(parse_expression(p.next().unwrap())),
                if_true: Box::new(parse_expression(p.next().unwrap())),
                if_false: Box::new(parse_expression(p.next().unwrap())),
            }
        }
        Rule::unary_operation => {
            let mut p = pair.into_inner();
            Expression::UnaryOperation {
                op: match p.next().unwrap().as_rule() {
                    Rule::op_minus => UnaryOperator::Minus,
                    Rule::op_not => UnaryOperator::Not,
                    unrecognized => unreachable!("parse unary operator: {:?}", unrecognized),
                },
                rhs: Box::new(parse_expression(p.next().unwrap())),
            }
        }
        Rule::reference => parse_reference(pair),
        Rule::global_reference => Expression::GlobalReference(pair.as_str().to_owned()),
        Rule::function => {
            let mut p = pair.into_inner();
            let name = p.next().unwrap().as_str().to_owned();
            p = p.next().unwrap().into_inner();
            Expression::Function {
                name,
                parameters: p.map(parse_expression).collect(),
            }
        }
        Rule::list_literal => Expression::List(pair.into_inner().map(parse_expression).collect()),
        Rule::map_literal => {
            let mut map = HashMap::new();
            for entry in pair.into_inner() {
                let mut p = entry.into_inner();
                let quoted_key = p.next().unwrap().into_inner().next().unwrap();
                let key = parse_quoted_string(quoted_key.into_inner().next().unwrap());
                let value = parse_expression(p.next().unwrap().into_inner().next().unwrap());
                map.insert(key, value);
            }
            Expression::Map(map)
        }
        unrecognized => unreachable!("parse expression: {:?}", unrecognized),
    }
}

fn parse_reference(pair: Pair<Rule>) -> Expression {
    let mut referent = None;
    let mut references = Vec::new();
    fn parse_name(p: Pair<Rule>) -> String {
        p.into_inner().next().unwrap().as_str().to_owned()
    }
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::variable => referent = Some(Referent::Variable(parse_name(p))),
            Rule::injected_data => referent = Some(Referent::Injected(parse_name(p))),
            Rule::bracketed_reference => references.push(Reference::Bracketed(parse_expression(
                p.into_inner().next().unwrap(),
            ))),
            Rule::dotted_reference => references.push(Reference::Dotted(parse_reference_key(
                p.into_inner().next().unwrap(),
            ))),
            Rule::question_bracketed_reference => references.push(Reference::QuestionBracketed(
                parse_expression(p.into_inner().next().unwrap()),
            )),
            Rule::question_dotted_reference => references.push(Reference::QuestionDotted(
                parse_reference_key(p.into_inner().next().unwrap()),
            )),
            unrecognized => unreachable!("parse reference: {:?}", unrecognized),
        }
    }
    Expression::DataReference {
        referent: referent.expect("expecting referent"),
        references,
    }
}

fn parse_reference_key(pair: Pair<Rule>) -> ReferenceKey {
    let p = pair.into_inner().next().expect("expecting reference key");
    match p.as_rule() {
        Rule::whole_number => ReferenceKey::Number(p.as_str().parse().unwrap()),
        Rule::name => ReferenceKey::Name(p.as_str().to_owned()),
        unrecognized => unreachable!("parse reference key: {:?}", unrecognized),
    }
}

fn parse_binary_operator(pair: &Pair<Rule>) -> BinaryOperator {
    match pair.as_rule() {
        Rule::op_minus => BinaryOperator::Minus,
        Rule::op_plus => BinaryOperator::Plus,
        Rule::op_times => BinaryOperator::Times,
        Rule::op_divide => BinaryOperator::Divide,
        Rule::op_modulo => BinaryOperator::Modulo,
        Rule::op_less => BinaryOperator::Less,
        Rule::op_le => BinaryOperator::LessEquals,
        Rule::op_greater => BinaryOperator::Greater,
        Rule::op_ge => BinaryOperator::GreaterEquals,
        Rule::op_equals => BinaryOperator::Equals,
        Rule::op_ne => BinaryOperator::NotEquals,
        Rule::op_and => BinaryOperator::And,
        Rule::op_or => BinaryOperator::Or,
        Rule::op_elvis => BinaryOperator::Elvis,
        unrecognized => unreachable!("parse operator: {:?}", unrecognized),
    }
}

fn build_binary_operation(lhs: Expression, ops: Vec<(BinaryOperator, Expression)>) -> Expression {
    use lazy_static::lazy_static;
    use std::collections::HashMap;
    lazy_static! {
        static ref ORDER_OF_OPS: HashMap<BinaryOperator, u8> = vec![
            (BinaryOperator::Times, 0),
            (BinaryOperator::Divide, 0),
            (BinaryOperator::Modulo, 0),
            (BinaryOperator::Plus, 1),
            (BinaryOperator::Minus, 1),
            (BinaryOperator::Less, 2),
            (BinaryOperator::Greater, 2),
            (BinaryOperator::LessEquals, 2),
            (BinaryOperator::GreaterEquals, 2),
            (BinaryOperator::Equals, 3),
            (BinaryOperator::NotEquals, 3),
            (BinaryOperator::And, 4),
            (BinaryOperator::Or, 5),
            (BinaryOperator::Elvis, 6),
        ]
        .into_iter()
        .collect();
    }
    let mut lhs = lhs;
    let mut ops = ops.into_iter().map(Some).collect::<Vec<_>>();
    while !ops.is_empty() {
        let (index, _) = ops
            .iter()
            .enumerate()
            .min_by_key(|(_, val)| ORDER_OF_OPS.get(&val.as_ref().unwrap().0).unwrap_or(&7))
            .unwrap();
        let (op, rhs) = ops.remove(index).unwrap();
        if index == 0 {
            lhs = Expression::BinaryOperation {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            };
        } else {
            let (left_op, left_expr) = ops[index - 1].take().unwrap();
            ops[index - 1] = Some((
                left_op,
                Expression::BinaryOperation {
                    lhs: Box::new(left_expr),
                    op,
                    rhs: Box::new(rhs),
                },
            ));
        }
    }
    lhs
}

fn parse_quoted_string(pair: Pair<Rule>) -> String {
    pair.into_inner().next().unwrap().as_str().to_owned()
}

fn parse_command(pair: Pair<Rule>) -> Command {
    match pair.as_rule() {
        Rule::msg_statement => Command::Msg {
            body: parse_message_body(pair),
        },
        Rule::print_statement => {
            let mut p = pair.into_inner();
            p.next(); // Get rid of the open tag.
            let expression = parse_expression(p.next().unwrap());
            let directives = p
                .next()
                .unwrap()
                .into_inner()
                .map(|pd| {
                    let mut pd = pd.into_inner();
                    PrintDirective {
                        name: pd.next().unwrap().as_str().to_owned(),
                        arguments: match pd.next() {
                            None => vec![],
                            Some(args) => args.into_inner().map(parse_expression).collect(),
                        },
                    }
                })
                .collect();
            Command::Print {
                expression,
                directives,
            }
        }
        unrecognized => unreachable!("parse command: {:?}", unrecognized),
    }
}

fn parse_message_body(pair: Pair<Rule>) -> MsgBody {
    let mut it = pair.into_inner();
    it.next().expect("expecting tag");
    it.next().expect("expecting attributes");
    let p = it.next().expect("expecting plural or block");
    match p.as_rule() {
        Rule::template_block => MsgBody::Block(parse_template_block(p)),
        Rule::msg_plural => {
            let mut expr = None;
            let mut cases = vec![];
            let mut default = None;
            for p in p.into_inner() {
                match p.as_rule() {
                    Rule::expression => expr = Some(parse_expression(p)),
                    Rule::plural_case => cases.push(parse_plural_case(p)),
                    Rule::plural_default => {
                        default = Some(parse_template_block(p.into_inner().next().unwrap()))
                    }
                    _ => {}
                }
            }
            MsgBody::Plural {
                expression: expr.expect("missing expression"),
                cases,
                default: default.expect("missing default"),
            }
        }
        unrecognized => unreachable!("parse msg body: {:?}", unrecognized),
    }
}

fn parse_plural_case(pair: Pair<Rule>) -> PluralCase {
    let mut expr = None;
    let mut body = None;
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::expression => expr = Some(parse_expression(p)),
            Rule::template_block => body = Some(parse_template_block(p)),
            unrecognized => unreachable!("parse plural case: {:?}", unrecognized),
        };
    }
    PluralCase {
        expression: expr.unwrap(),
        body: body.unwrap(),
    }
}
