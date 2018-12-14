use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct SoyFile {
    pub namespace: Namespace,
    pub aliases: Vec<Alias>,
    pub delpackage: Option<String>,
    pub templates: Vec<Template>,
}

#[derive(Debug, PartialEq)]
pub struct Namespace {
    pub name: String,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, PartialEq)]
pub struct Alias {
    pub from: String,
    pub to: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Template {
    pub name: String,
    pub body: TemplateBlock,
    pub soydoc_params: Vec<SoydocParam>,
}

pub type TemplateBlock = Vec<TemplateNode>;

#[derive(Debug, PartialEq)]
pub enum TemplateNode {
    RawText { value: String, newline: bool },
    Statement { command: Command, newline: bool },
}

#[derive(Debug, PartialEq)]
pub struct SoydocParam {
    pub name: String,
    pub required: bool,
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
