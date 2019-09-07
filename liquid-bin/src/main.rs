// Allow zero pointers for lazy_static. Otherwise clippy will complain.
#![allow(unknown_lints)]

use liquid;

use serde_json;
use serde_yaml;

use std::ffi;
use std::fs;
use std::io::Write;
use std::path;

use structopt::StructOpt;

#[derive(Copy, Clone, Debug, derive_more::Display, derive_more::From, derive_more::Constructor)]
#[display(fmt = "{}", msg)]
struct Error {
    msg: &'static str,
}

impl std::error::Error for Error {}

fn load_yaml(path: &path::Path) -> Result<liquid::value::Value, Box<dyn std::error::Error>> {
    let f = fs::File::open(path)?;
    serde_yaml::from_reader(f).map_err(|e| e.into())
}

fn load_json(path: &path::Path) -> Result<liquid::value::Value, Box<dyn std::error::Error>> {
    let f = fs::File::open(path)?;
    serde_json::from_reader(f).map_err(|e| e.into())
}

fn build_context(path: &path::Path) -> Result<liquid::value::Object, Box<dyn std::error::Error>> {
    let extension = path.extension().unwrap_or_else(|| ffi::OsStr::new(""));
    let value = if extension == ffi::OsStr::new("yaml") {
        load_yaml(path)
    } else if extension == ffi::OsStr::new("yaml") {
        load_json(path)
    } else {
        Err(Error::new("Unsupported file type"))?
    }?;
    let value = match value {
        liquid::value::Value::Object(o) => Ok(o),
        _ => Err(Error::new("File must be an object")),
    }?;

    Ok(value)
}

#[derive(StructOpt)]
struct Args {
    #[structopt(long, parse(from_os_str))]
    input: std::path::PathBuf,

    #[structopt(long, parse(from_os_str))]
    output: Option<std::path::PathBuf>,

    #[structopt(long, parse(from_os_str))]
    context: Option<std::path::PathBuf>,
}

fn run() -> Result<i32, Box<dyn std::error::Error>> {
    let args = Args::from_args();

    let parser = liquid::ParserBuilder::with_liquid()
        .extra_filters()
        .jekyll_filters()
        .build()
        .expect("should succeed without partials");
    let template = parser.parse_file(&args.input)?;

    let data = args
        .context
        .as_ref()
        .map(|p| build_context(p.as_path()))
        .map_or(Ok(None), |r| r.map(Some))?
        .unwrap_or_else(liquid::value::Object::new);
    let output = template.render(&data)?;
    match args.output {
        Some(path) => {
            let mut out = fs::File::create(path)?;
            out.write_all(output.as_bytes())?;
        }
        None => {
            println!("{}", output);
        }
    }

    Ok(0)
}

fn main() {
    let code = run().unwrap();
    std::process::exit(code);
}
