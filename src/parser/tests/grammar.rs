use super::{Rule::*, *};

macro_rules! assert_matches {
    ($rule:expr, $input:expr) => {
        assert_eq!(
            SoyParser::parse($rule, $input)
                .map_err(|e| format!("{}", e))
                .expect(&format!("{:?} should parse:\n{:?}, ", $rule, $input))
                .last()
                .unwrap()
                .into_span()
                .end(),
            $input.len(),
            "{:?} should consume all input: {:?}",
            $rule,
            $input
        );
    };
}

macro_rules! assert_parses {
    ($rule:expr, $input:expr) => {
        assert!(
            SoyParser::parse($rule, $input).is_ok(),
            "{:?} {:?}",
            $rule,
            $input
        );
    };
}

macro_rules! assert_fails {
    ($rule:expr, $input:expr, $reason:expr) => {
        assert!(
            SoyParser::parse($rule, $input).is_err(),
            "{:?} should fail: {:?}",
            $reason,
            $input
        );
    };
}

#[test]
fn test_minimal_file() {
    let input = "// file comment
{namespace foo.bar}
/**
 *
 */
{template .baz}
{/template}";
    parse(input).expect("should work");
}

#[test]
fn test_namespace() {
    assert_matches!(namespace, "{namespace foo}\n");
    assert_matches!(namespace, "{namespace a.b.c}\n");
    assert_matches!(namespace, "{namespace a.b.c x=\"y\" zz='ZZ'}\n");
    assert_fails!(namespace, "{namespace a.b.c /}\n", "not self closing");
    assert_fails!(namespace, "{namespace}\n", "missing name");
    assert_fails!(namespace, "{namespace a.b.c} foo", "needs its own line");
    assert_parses!(namespace, "{namespace a.b.c}\nfoo");
}

#[test]
fn test_delpackage() {
    assert_matches!(delpackage, "{delpackage a.b.c}\n");
    assert_fails!(delpackage, "{delpackage}\n", "missing name");
}

#[test]
fn test_alias() {
    assert_matches!(alias, "{alias a.b.c}\n");
    assert_matches!(alias, "{alias a.b.c as d}\n");
    assert_fails!(alias, "{alias}\n", "missing name");
    assert_fails!(alias, "{alias as d}\n", "missing name");
    assert_fails!(alias, "{alias a.b.c x='y'}\n", "attributes aren't allowed");
}

#[test]
fn test_soydoc() {
    assert_matches!(soydoc, "/**\n * @param foo\n * @param bar description\n */");
    assert_matches!(soydoc, "/**\n *    @param? foo\n *@param foo */");
    assert_fails!(soydoc, "/**\n *    @param? foo\n", "missing closing tag");
    assert_fails!(soydoc, "/*\n *    @param? foo\n*/", "not soydoc");
}

#[test]
fn test_template() {
    assert_matches!(template, "/** */\n{template .foo}{/template}");
    assert_fails!(template, "{template .foo}{/template}", "missing soydoc");
}

#[test]
fn test_specials() {
    assert_matches!(special_sp, "{sp}");
    assert_matches!(special_lb, "{lb}");
    assert_matches!(special_rb, "{rb}");
    assert_matches!(special_nil, "{nil}");
    assert_matches!(special_return, "{\\r}");
    assert_matches!(special_newline, "{\\n}");
    assert_matches!(special_tab, "{\\t}");
}

#[test]
fn test_expressions() {
    // valid expressions
    [
        "$foo",
        "$baz?.0.bar['baz\\'']",
        "global.name",
        "$ij.bar?[0].baz[$foobar.bar].bar[true]",
        "$ij.foo[3 * $baz]?.bar",
        "-56",
        "5.3",
        "5e-5",
        "-5e-2",
        "5.0e-3",
        "'some str'",
        "[true, false, null, boop, $baz[3], $bar]",
        "[]",
        "[:]",
        "['foo': false, 'bar': [3, 4, 5]]",
        "foo(null, boo, 38.5)",
        "boop([a, b, c])",
        "(-(($foo * 5 - 3) / 4) + (5 * 4 * -$foo)) ? $foo : $bar ?: $baz",
        "5 * $foo < 27 ? 'foo' : 'bar'",
    ]
    .iter()
    .for_each(|expr| assert_matches!(expression, expr));

    // expected failures
    [
        (".3", "0 before decimal point required"),
        ("\"bad str\"", "double quoted strings not allowed"),
        ("()", "empty parens"),
        ("[1, 2, 3", "unclosed list"),
    ]
    .iter()
    .for_each(|(expr, reason)| assert_fails!(expression, expr, reason));
}

#[test]
fn test_msg() {
    assert_matches!(msg_statement, "{msg foo=\"bar\"}fun <span>{/msg}");
    assert_matches!(msg_statement, "{msg}{/msg}");
    assert_matches!(msg_statement, "{msg}foo{/msg}");
    assert_matches!(msg_statement, "{msg}{plural 1}{default}{/plural}{/msg}");
    assert_matches!(msg_statement, "{msg}hi {$name}{/msg}");
    assert_fails!(
        msg_statement,
        "{msg}foo{plural 1}{default}{/plural}{/msg}",
        "can't have both body and plural"
    )
}

#[test]
fn test_plural() {
    assert_matches!(
        msg_plural,
        "{plural length(name)}\n {case 1}\n  Foo\n {default}\n  Bar \n{/plural}"
    );
    assert_matches!(msg_plural, "{plural 7}{default}Bar{/plural}");
    assert_fails!(
        msg_plural,
        "{plural}{case 1}a{default}{/plural}",
        "missing expression"
    );
    assert_fails!(
        msg_plural,
        "{plural 5}{case 1}a{/plural}",
        "missing default"
    );
}

#[test]
fn test_print() {
    assert_matches!(print_statement, "{$foo}");
    assert_matches!(print_statement, "{print $foo}");
    assert_matches!(print_statement, "{ $foo |changeNewlineToBr}");
    assert_matches!(
        print_statement,
        "{$foo['bar'].baz |changeNewlineToBr |truncate:8,false}"
    );
}
