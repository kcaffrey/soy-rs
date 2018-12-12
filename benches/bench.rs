use criterion::{criterion_group, criterion_main, Criterion};
use soy::Tofu;

fn simple_benchmark(c: &mut Criterion) {
    let tofu = Tofu::with_string_template(HELLO_WORLD).unwrap();
    c.bench_function("hello world", move |b| {
        b.iter(|| tofu.render("benches.hello"))
    });
}

criterion_group!(benches, simple_benchmark);
criterion_main!(benches);

static HELLO_WORLD: &str = "
{namespace benches}

/**
 * Hello world
 */
{template .hello}
Hello world
{/template}
";
