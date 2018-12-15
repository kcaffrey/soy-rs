use super::{Rule::*, *};
use pest::Parser;

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

macro_rules! raw_text {
    ($text:expr) => {
        TemplateNode::RawText {
            value: $text.to_owned(),
            newline: false,
        }
    };
    ($text:expr, $linebreak:expr) => {
        TemplateNode::RawText {
            value: $text.to_owned(),
            newline: $linebreak,
        }
    };
}

macro_rules! attributes {
    ($($name:ident = $value:expr),*) => {
        [$((stringify!($name).to_owned(), $value.to_owned()),)*]
            .iter()
            .cloned()
            .collect::<HashMap<String, String>>()
    };
}

#[test]
fn test_soyfile() {
    let cases = &[
        (
            "{namespace foo}\n/** */{template .bar}foo{/template}",
            SoyFile {
                namespace: Namespace {
                    name: "foo".to_owned(),
                    attributes: attributes!(),
                },
                delpackage: None,
                aliases: vec![],
                templates: vec![Template {
                    name: "bar".to_owned(),
                    body: vec![raw_text!("foo")],
                    soydoc_params: vec![],
                }],
            },
        ),
        (
            "// foo\n//bar\n\n{delpackage a}\n{namespace foo}\n{alias b}\n/** */{template .bar}foo{/template}",
            SoyFile {
                namespace: Namespace {
                    name: "foo".to_owned(),
                    attributes: attributes!(),
                },
                delpackage: Some("a".to_owned()),
                aliases: vec![Alias {
                    from: "b".to_owned(),
                    to: None,
                }],
                templates: vec![Template {
                    name: "bar".to_owned(),
                    body: vec![raw_text!("foo")],
                    soydoc_params: vec![],
                }],
            },
        ),
    ];

    cases.iter().for_each(|(input, expected)| {
        assert_eq!(
            parse!(input, (soy_file, parse_soyfile)).unwrap(),
            *expected,
            "\n{}",
            input
        );
    });
}

#[test]
fn test_namespace() {
    let cases = &[
        (
            "{namespace foo}\n",
            Namespace {
                name: "foo".to_owned(),
                attributes: attributes!(),
            },
        ),
        (
            "{namespace a.b.c bar='baz' a=\"b\"}\n",
            Namespace {
                name: "a.b.c".to_owned(),
                attributes: attributes!(bar = "baz", a = "b"),
            },
        ),
    ];

    cases.iter().for_each(|(input, expected)| {
        assert_eq!(
            parse!(input, (namespace, parse_namespace)),
            *expected,
            "\n{}",
            input
        );
    });
}

#[test]
fn test_alias() {
    let cases = &[
        (
            "{alias foo.bar}\n",
            Alias {
                from: "foo.bar".to_owned(),
                to: None,
            },
        ),
        (
            "{alias foo.bar as foobar}\n",
            Alias {
                from: "foo.bar".to_owned(),
                to: Some("foobar".to_owned()),
            },
        ),
    ];

    cases.iter().for_each(|(input, expected)| {
        assert_eq!(
            parse!(input, (alias, parse_alias)),
            *expected,
            "\n{}",
            input
        );
    });
}

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
        assert_eq!(
            parse!(input, (expression, parse_expression)),
            *expected,
            "\n{}",
            input
        );
    });
}

#[test]
fn test_specials() {
    let cases = &[
        ("{sp}", TemplateNode::Special(" ".to_owned())),
        ("{nil}", TemplateNode::Special("".to_owned())),
        ("{lb}", TemplateNode::Special("{".to_owned())),
        ("{rb}", TemplateNode::Special("}".to_owned())),
        ("{\\r}", TemplateNode::Special("\\r".to_owned())),
        ("{\\n}", TemplateNode::Special("\\n".to_owned())),
        ("{\\t}", TemplateNode::Special("\\t".to_owned())),
    ];
    for (input, expected) in cases {
        assert_eq!(
            parse!(input, (special, parse_special)),
            *expected,
            "\n{}",
            input
        );
    }
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
                    body: vec![raw_text!("foo")]
                }],
                default: vec![raw_text!("bar")]
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
        assert_eq!(
            parse!(input, (print_statement, parse_command)),
            *expected,
            "\n{}",
            input
        );
    });
}

#[test]
fn test_template() {
    // TODO: attributes?
    let cases = &[
            (
                "/** */{template .foo}{/template}",
                Template {
                    name: "foo".to_owned(),
                    body: TemplateBlock::new(),
                    soydoc_params: vec![],
                },
            ),
            (
                "/**\n * @param foo a foo\n * @param? bar\n */\n{template .foo}{$foo}{sp}{/template}",
                Template {
                    name: "foo".to_owned(),
                    body: vec![
                        TemplateNode::Statement {
                            command: Command::Print {
                                expression: variable!("foo"),
                                directives: vec![],
                            },
                            newline: false
                        }, 
                        TemplateNode::Special(" ".to_owned())
                    ],
                    soydoc_params: vec![
                        SoydocParam {
                            name: "foo".to_owned(),
                            required: true,
                        },
                        SoydocParam {
                            name: "bar".to_owned(),
                            required: false,
                        },
                    ],
                },
            ),
            (
                // Demonstrating comments and whitespace stripping
                "/** */{template .foo} First // comment \n  Second<br>\n\n  // A comment \n  <i>Third</i>\n{/template}",
                Template {
                    name: "foo".to_owned(),
                    body: vec![
                        raw_text!("First", true),
                        raw_text!("Second<br>", true),
                        raw_text!("<i>Third</i>", true),
                    ],
                    soydoc_params: vec![],
                },
            ),
            (
                // Demonstrating various forms of comments
                "/** */{template .foo}Foo // foooo\n Bar /* comment \n foo */\n /* lks */ Baz{/template}",
                Template {
                    name: "foo".to_owned(),
                    body: vec![
                        raw_text!("Foo", true),
                        raw_text!("Bar", true),
                        raw_text!("Baz", false),
                    ],
                    soydoc_params: vec![],
                },
            ),
        ];

    cases.iter().for_each(|(input, expected)| {
        assert_eq!(
            parse!(input, (template, parse_template)),
            *expected,
            "\n{}",
            input
        );
    });
}
