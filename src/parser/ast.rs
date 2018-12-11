use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct SoyFile {
    pub templates: Vec<Template>,
}

#[derive(Debug, PartialEq)]
pub struct Template {
    pub name: String,
    pub body: TemplateBlock,
}

pub type TemplateBlock = Vec<TemplateNode>;

#[derive(Debug, PartialEq)]
pub enum TemplateNode {
    RawText(String),
    Statement(Command),
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Msg {
        body: MsgBody,
    },
    Print {
        expression: Expression,
        directives: Vec<PrintDirective>,
    },
}

#[derive(Debug, PartialEq)]
pub enum MsgBody {
    Plural {
        expression: Expression,
        cases: Vec<PluralCase>,
        default: TemplateBlock,
    },
    Block(TemplateBlock),
}

#[derive(Debug, PartialEq)]
pub struct PluralCase {
    pub expression: Expression,
    pub body: TemplateBlock,
}

#[derive(Debug, PartialEq)]
pub struct PrintDirective {
    pub name: String,
    pub arguments: Vec<Expression>,
}

#[derive(Debug, PartialEq)]
pub enum Expression {
    Null,
    Boolean(bool),
    Float(f64),
    Integer(i64),
    String(String),
    List(Vec<Expression>),
    Map(HashMap<String, Expression>),
    Function {
        name: String,
        parameters: Vec<Expression>,
    },
    DataReference {
        referent: Referent,
        references: Vec<Reference>,
    },
    GlobalReference(String),
    BinaryOperation {
        lhs: Box<Expression>,
        op: BinaryOperator,
        rhs: Box<Expression>,
    },
    UnaryOperation {
        op: UnaryOperator,
        rhs: Box<Expression>,
    },
    TernaryOperation {
        condition: Box<Expression>,
        if_true: Box<Expression>,
        if_false: Box<Expression>,
    },
}

#[derive(Debug, PartialEq, Hash, Eq)]
pub enum BinaryOperator {
    Plus,
    Minus,
    Times,
    Divide,
    Modulo,
    Less,
    LessEquals,
    Greater,
    GreaterEquals,
    Equals,
    NotEquals,
    And,
    Or,
    Elvis,
}

#[derive(Debug, PartialEq)]
pub enum UnaryOperator {
    Minus,
    Not,
}

#[derive(Debug, PartialEq)]
pub enum Referent {
    Variable(String),
    Injected(String),
}

#[derive(Debug, PartialEq)]
pub enum Reference {
    Dotted(ReferenceKey),
    QuestionDotted(ReferenceKey),
    Bracketed(Expression),
    QuestionBracketed(Expression),
}

#[derive(Debug, PartialEq)]
pub enum ReferenceKey {
    Number(usize),
    Name(String),
}

use crate::parser::Rule;
use pest::iterators::Pair;

fn parse_template_block(pair: Pair<Rule>) -> TemplateBlock {
    pair.into_inner()
        .map(|p| match p.as_rule() {
            Rule::statement => {
                TemplateNode::Statement(parse_command(p.into_inner().next().unwrap()))
            }
            Rule::raw_text => TemplateNode::RawText(p.as_str().to_owned()),
            unrecognized => unreachable!("parse template block: {:?}", unrecognized),
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

#[cfg(test)]
#[macro_use]
mod test_macros {
    macro_rules! parse {
        ($input:expr, ($rule:expr, $fn:ident)) => {
            $fn(SoyParser::parse($rule, $input).unwrap().next().unwrap())
        };
    }

    macro_rules! bin_op {
        ($lhs:expr, $op:tt, $rhs:expr) => {
            Expression::BinaryOperation {
                lhs: Box::new($lhs),
                op: BinaryOperator::$op,
                rhs: Box::new($rhs),
            }
        };
    }

    macro_rules! variable {
        ($name:expr) => {
            Expression::DataReference {
                referent: Referent::Variable($name.to_owned()),
                references: vec![],
            }
        };
    }

    macro_rules! int {
        ($val:expr) => {
            Expression::Integer($val)
        };
    }

    macro_rules! list {
        ($($item:expr),*) => {
            Expression::List(vec![$($item,)*])
        };
    }

    macro_rules! map {
        ($(($key:expr, $val:expr)),*) => {
            Expression::Map(vec![$(($key.to_owned(), $val),)*].into_iter().collect())
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Rule::*, SoyParser};
    use pest::Parser;

    #[test]
    fn test_expressions() {
        let cases = &[
            ("null", Expression::Null),
            ("true", Expression::Boolean(true)),
            ("57", int!(57)),
            ("56.3", Expression::Float(56.3)),
            ("4.1e27", Expression::Float(4.1e27)),
            ("'foo'", Expression::String("foo".to_owned())),
            ("$foo", variable!("foo")),
            ("foobar", Expression::GlobalReference("foobar".to_owned())),
            (
                "$foo.bar",
                Expression::DataReference {
                    referent: Referent::Variable("foo".to_owned()),
                    references: vec![Reference::Dotted(ReferenceKey::Name("bar".to_owned()))],
                },
            ),
            (
                "$ij.foo[3 * $baz]?.bar",
                Expression::DataReference {
                    referent: Referent::Injected("foo".to_owned()),
                    references: vec![
                        Reference::Bracketed(bin_op!(int!(3), Times, variable!("baz"))),
                        Reference::QuestionDotted(ReferenceKey::Name("bar".to_owned())),
                    ],
                },
            ),
            (
                "5 * (4 - 3 / 1) * 2",
                bin_op!(
                    bin_op!(
                        int!(5),
                        Times,
                        bin_op!(int!(4), Minus, bin_op!(int!(3), Divide, int!(1)))
                    ),
                    Times,
                    int!(2)
                ),
            ),
            (
                "5 * $foo < 27 ? 'foo' : $baz + 'bar'",
                Expression::TernaryOperation {
                    condition: Box::new(bin_op!(
                        bin_op!(int!(5), Times, variable!("foo")),
                        Less,
                        int!(27)
                    )),
                    if_true: Box::new(Expression::String("foo".to_owned())),
                    if_false: Box::new(bin_op!(
                        variable!("baz"),
                        Plus,
                        Expression::String("bar".to_owned())
                    )),
                },
            ),
            (
                "3 / -$bar",
                bin_op!(
                    int!(3),
                    Divide,
                    Expression::UnaryOperation {
                        op: UnaryOperator::Minus,
                        rhs: Box::new(variable!("bar"))
                    }
                ),
            ),
            (
                "[4,$foo, 5*7]",
                list!(int!(4), variable!("foo"), bin_op!(int!(5), Times, int!(7))),
            ),
            (
                "['foo': 4, 'bar': [5, $baz]]",
                map!(("foo", int!(4)), ("bar", list!(int!(5), variable!("baz")))),
            ),
            (
                "foobar(5, $baz * 2)",
                Expression::Function {
                    name: "foobar".to_owned(),
                    parameters: vec![int!(5), bin_op!(variable!("baz"), Times, int!(2))],
                },
            ),
        ];

        cases.iter().for_each(|(input, expected)| {
            assert_eq!(parse!(input, (expression, parse_expression)), *expected);
        });
    }

    #[test]
    fn test_msg() {
        assert_eq!(
            parse!(
                "{msg}{plural $foo}{case 5} foo{default}bar{/plural}{/msg}",
                (msg_statement, parse_command)
            ),
            Command::Msg {
                body: MsgBody::Plural {
                    expression: Expression::DataReference {
                        referent: Referent::Variable("foo".to_owned()),
                        references: vec![],
                    },
                    cases: vec![PluralCase {
                        expression: Expression::Integer(5),
                        body: vec![TemplateNode::RawText("foo".to_owned())]
                    }],
                    default: vec![TemplateNode::RawText("bar".to_owned())]
                }
            }
        )
    }

    #[test]
    fn test_print() {
        let cases = &[
            (
                "{$foo}",
                Command::Print {
                    expression: variable!("foo"),
                    directives: vec![],
                },
            ),
            (
                "{print $foo}",
                Command::Print {
                    expression: variable!("foo"),
                    directives: vec![],
                },
            ),
            (
                "{$foo.baz |changeNewlineToBr |truncate:8,false}",
                Command::Print {
                    expression: Expression::DataReference {
                        referent: Referent::Variable("foo".to_owned()),
                        references: vec![Reference::Dotted(ReferenceKey::Name("baz".to_owned()))],
                    },
                    directives: vec![
                        PrintDirective {
                            name: "changeNewlineToBr".to_owned(),
                            arguments: vec![],
                        },
                        PrintDirective {
                            name: "truncate".to_owned(),
                            arguments: vec![int!(8), Expression::Boolean(false)],
                        },
                    ],
                },
            ),
        ];
        cases.iter().for_each(|(input, expected)| {
            assert_eq!(parse!(input, (print_statement, parse_command)), *expected);
        });
    }
}
