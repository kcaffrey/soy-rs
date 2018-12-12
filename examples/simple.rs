use soy::Tofu;

static TEMPLATE: &str = "
{namespace example}

/**
 * Says hello to the world.
 */
{template .helloWorld}
  Hello world!
{/template}
";

fn main() -> Result<(), String> {
    let tofu = Tofu::with_string_template(TEMPLATE)?;
    println!(
        "{}",
        tofu.render("example.helloWorld")
            .map_err(|e| format!("{}", e))?
    );
    Ok(())
}
