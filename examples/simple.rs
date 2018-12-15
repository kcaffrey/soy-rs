use soy::Tofu;
use std::error::Error;

static TEMPLATE: &str = "
{namespace example}

/**
 * Says hello to the world.
 */
{template .helloWorld}
  Hello world!
{/template}
";

fn main() -> Result<(), Box<Error>> {
    let tofu = Tofu::with_string_template(TEMPLATE)?;
    println!("{}", tofu.render("example.helloWorld")?);
    Ok(())
}
