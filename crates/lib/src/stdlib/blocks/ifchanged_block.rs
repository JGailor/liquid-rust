use std::io::Write;

use liquid_core::error::{ResultLiquidExt, ResultLiquidReplaceExt};
use liquid_core::Language;
use liquid_core::Renderable;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::Template;
use liquid_core::{BlockReflection, ParseBlock, TagBlock, TagTokenIter};

#[derive(Debug)]
struct IfChanged {
    if_changed: Template,
}

impl IfChanged {
    fn trace(&self) -> String {
        "{{% ifchanged %}}".to_owned()
    }
}

impl Renderable for IfChanged {
    fn render_to(&self, writer: &mut dyn Write, runtime: &mut Runtime<'_>) -> Result<()> {
        let mut rendered = Vec::new();
        self.if_changed
            .render_to(&mut rendered, runtime)
            .trace_with(|| self.trace().into())?;

        let rendered = String::from_utf8(rendered).expect("render only writes UTF-8");
        if runtime.get_register_mut::<State>().has_changed(&rendered) {
            write!(writer, "{}", rendered).replace("Failed to render")?;
        }

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct IfChangedBlock;

impl IfChangedBlock {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BlockReflection for IfChangedBlock {
    fn start_tag(&self) -> &str {
        "ifchanged"
    }

    fn end_tag(&self) -> &str {
        "endifchanged"
    }

    fn description(&self) -> &str {
        ""
    }
}

impl ParseBlock for IfChangedBlock {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        mut tokens: TagBlock<'_, '_>,
        options: &Language,
    ) -> Result<Box<dyn Renderable>> {
        // no arguments should be supplied, trying to supply them is an error
        arguments.expect_nothing()?;

        let if_changed = Template::new(tokens.parse_all(options)?);

        tokens.assert_empty();
        Ok(Box::new(IfChanged { if_changed }))
    }

    fn reflection(&self) -> &dyn BlockReflection {
        self
    }
}

/// Remembers the content of the last rendered `ifstate` block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct State {
    last_rendered: Option<String>,
}

impl State {
    /// Checks whether or not a new rendered `&str` is different from
    /// `last_rendered` and updates `last_rendered` value to the new value.
    fn has_changed(&mut self, rendered: &str) -> bool {
        let has_changed = if let Some(last_rendered) = &self.last_rendered {
            last_rendered != rendered
        } else {
            true
        };
        self.last_rendered = Some(rendered.to_owned());

        has_changed
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use liquid_core::compiler;
    use liquid_core::interpreter;

    use crate::stdlib;

    fn options() -> Language {
        let mut options = Language::default();
        options
            .blocks
            .register("ifchanged".to_string(), IfChangedBlock.into());
        options
            .blocks
            .register("for".to_string(), stdlib::ForBlock.into());
        options
            .blocks
            .register("if".to_string(), stdlib::IfBlock.into());
        options
    }

    #[test]
    fn test_ifchanged_block() {
        let text = concat!(
            "{% for a in (0..10) %}",
            "{% ifchanged %}",
            "\nHey! ",
            "{% if a > 5 %}",
            "Numbers are now bigger than 5!",
            "{% endif %}",
            "{% endifchanged %}",
            "{% endfor %}",
        );
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut runtime = Runtime::new();
        let output = template.render(&mut runtime).unwrap();
        assert_eq!(output, "\nHey! \nHey! Numbers are now bigger than 5!");
    }
}
