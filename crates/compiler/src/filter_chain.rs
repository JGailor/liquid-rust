use std::fmt;
use std::io::Write;

use itertools;

use super::Filter;
use liquid_error::{Result, ResultLiquidExt, ResultLiquidReplaceExt};
use liquid_interpreter::Expression;
use liquid_interpreter::Renderable;
use liquid_interpreter::Runtime;
use liquid_value::{ValueCow, ValueView};

/// A `Value` expression.
#[derive(Debug)]
pub struct FilterChain {
    entry: Expression,
    filters: Vec<Box<dyn Filter>>,
}

impl FilterChain {
    /// Create a new expression.
    pub fn new(entry: Expression, filters: Vec<Box<dyn Filter>>) -> Self {
        Self { entry, filters }
    }

    /// Process `Value` expression within `runtime`'s stack.
    pub fn evaluate<'s>(&'s self, runtime: &'s Runtime) -> Result<ValueCow<'s>> {
        // take either the provided value or the value from the provided variable
        let mut entry = self.entry.evaluate(runtime)?;

        // apply all specified filters
        for filter in &self.filters {
            entry = ValueCow::Owned(
                filter
                    .evaluate(entry.as_view(), runtime)
                    .trace("Filter error")
                    .context_key("filter")
                    .value_with(|| format!("{}", filter).into())
                    .context_key("input")
                    .value_with(|| format!("{}", entry.source()).into())?,
            );
        }

        Ok(entry)
    }
}

impl fmt::Display for FilterChain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} | {}",
            self.entry,
            itertools::join(&self.filters, " | ")
        )
    }
}

impl Renderable for FilterChain {
    fn render_to(&self, writer: &mut dyn Write, runtime: &mut Runtime) -> Result<()> {
        let entry = self.evaluate(runtime)?;
        write!(writer, "{}", entry.render()).replace("Failed to render")?;
        Ok(())
    }
}
