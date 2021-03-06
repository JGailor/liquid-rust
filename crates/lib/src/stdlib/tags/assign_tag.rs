use std::io::Write;

use liquid_core::compiler::FilterChain;
use liquid_core::error::ResultLiquidExt;
use liquid_core::Language;
use liquid_core::Renderable;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{ParseTag, TagReflection, TagTokenIter};

#[derive(Debug)]
struct Assign {
    dst: String,
    src: FilterChain,
}

impl Assign {
    fn trace(&self) -> String {
        format!("{{% assign {} = {}%}}", self.dst, self.src)
    }
}

impl Renderable for Assign {
    fn render_to(&self, _writer: &mut dyn Write, runtime: &mut Runtime<'_>) -> Result<()> {
        let value = self
            .src
            .evaluate(runtime)
            .trace_with(|| self.trace().into())?
            .into_owned();
        runtime.stack_mut().set_global(self.dst.to_owned(), value);
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct AssignTag;

impl AssignTag {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TagReflection for AssignTag {
    fn tag(&self) -> &'static str {
        "assign"
    }

    fn description(&self) -> &'static str {
        ""
    }
}

impl ParseTag for AssignTag {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        options: &Language,
    ) -> Result<Box<dyn Renderable>> {
        let dst = arguments
            .expect_next("Identifier expected.")?
            .expect_identifier()
            .into_result()?
            .to_string();

        arguments
            .expect_next("Assignment operator \"=\" expected.")?
            .expect_str("=")
            .into_result_custom_msg("Assignment operator \"=\" expected.")?;

        let src = arguments
            .expect_next("FilterChain expected.")?
            .expect_filter_chain(options)
            .into_result()?;

        // no more arguments should be supplied, trying to supply them is an error
        arguments.expect_nothing()?;

        Ok(Box::new(Assign { dst, src }))
    }

    fn reflection(&self) -> &dyn TagReflection {
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use liquid_core::compiler;
    use liquid_core::interpreter;
    use liquid_core::value::Scalar;
    use liquid_core::value::Value;

    use crate::stdlib;

    fn options() -> Language {
        let mut options = Language::default();
        options
            .tags
            .register("assign".to_string(), AssignTag.into());
        options
            .blocks
            .register("if".to_string(), stdlib::IfBlock.into());
        options
            .blocks
            .register("for".to_string(), stdlib::ForBlock.into());
        options
    }

    #[test]
    fn assign() {
        let options = options();
        let template = compiler::parse("{% assign freestyle = false %}{{ freestyle }}", &options)
            .map(interpreter::Template::new)
            .unwrap();

        let mut runtime = Runtime::new();

        let output = template.render(&mut runtime).unwrap();
        assert_eq!(output, "false");
    }

    #[test]
    fn assign_array_indexing() {
        let text = concat!("{% assign freestyle = tags[1] %}", "{{ freestyle }}");
        let options = options();
        let template = compiler::parse(text, &options)
            .map(interpreter::Template::new)
            .unwrap();

        let mut runtime = Runtime::new();
        runtime.stack_mut().set_global(
            "tags",
            Value::Array(vec![
                Value::scalar("alpha"),
                Value::scalar("beta"),
                Value::scalar("gamma"),
            ]),
        );

        let output = template.render(&mut runtime).unwrap();
        assert_eq!(output, "beta");
    }

    #[test]
    fn assign_object_indexing() {
        let text = concat!(
            r#"{% assign freestyle = tags["greek"] %}"#,
            "{{ freestyle }}"
        );
        let options = options();
        let template = compiler::parse(text, &options)
            .map(interpreter::Template::new)
            .unwrap();

        let mut runtime = Runtime::new();
        runtime.stack_mut().set_global(
            "tags",
            Value::Object(
                vec![("greek".into(), Value::scalar("alpha"))]
                    .into_iter()
                    .collect(),
            ),
        );

        let output = template.render(&mut runtime).unwrap();
        assert_eq!(output, "alpha");
    }

    #[test]
    fn assign_in_loop_persists_on_loop_exit() {
        let text = concat!(
            "{% assign freestyle = false %}",
            "{% for t in tags %}{% if t == 'freestyle' %}",
            "{% assign freestyle = true %}",
            "{% endif %}{% endfor %}",
            "{% if freestyle %}",
            "<p>Freestyle!</p>",
            "{% endif %}"
        );

        let options = options();
        let template = compiler::parse(text, &options)
            .map(interpreter::Template::new)
            .unwrap();

        // test one: no matching value in `tags`
        {
            let mut runtime = Runtime::new();
            runtime.stack_mut().set_global(
                "tags",
                Value::Array(vec![
                    Value::scalar("alpha"),
                    Value::scalar("beta"),
                    Value::scalar("gamma"),
                ]),
            );

            let output = template.render(&mut runtime).unwrap();
            assert_eq!(
                runtime.stack().get(&[Scalar::new("freestyle")]).unwrap(),
                false
            );
            assert_eq!(output, "");
        }

        // test two: matching value in `tags`
        {
            let mut runtime = Runtime::new();
            runtime.stack_mut().set_global(
                "tags",
                Value::Array(vec![
                    Value::scalar("alpha"),
                    Value::scalar("beta"),
                    Value::scalar("freestyle"),
                    Value::scalar("gamma"),
                ]),
            );

            let output = template.render(&mut runtime).unwrap();
            assert_eq!(
                runtime.stack().get(&[Scalar::new("freestyle")]).unwrap(),
                true
            );
            assert_eq!(output, "<p>Freestyle!</p>");
        }
    }
}
