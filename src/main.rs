use std::io;
use std::io::{Read, Write};
use std::process;

use clap::{App, Arg, ArgMatches, SubCommand};
use mdbook::errors::Error;
use mdbook::preprocess::{CmdPreprocessor, Preprocessor};
use serde_json;

use mdbook_fix_cjk_spacing::{join_cjk_spacing, FixCJKSpacing};

pub fn make_app() -> App<'static, 'static> {
    App::new("mdbook-fix-cjk-spacing")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A mdbook preprocessor that will remove line breaks between CJK lines")
        .subcommand(
            SubCommand::with_name("supports")
                .arg(Arg::with_name("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
        .subcommand(
            SubCommand::with_name("raw")
                .about("Process raw markdown files, e.g. `cat mark.md | mdbook-fix-cjk-spacing`"),
        )
}

fn main() {
    let matches = make_app().get_matches();

    // Users will want to construct their own preprocessor here
    let preprocessor = FixCJKSpacing::new();

    if let Some(sub_args) = matches.subcommand_matches("supports") {
        handle_supports(&preprocessor, sub_args);
    } else if let Some(sub_args) = matches.subcommand_matches("raw") {
        let _ = handle_raw(sub_args);
    } else if let Err(e) = handle_preprocessing(&preprocessor) {
        eprintln!("{}", e);
        process::exit(1);
    }
}

fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<(), Error> {
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

    if ctx.mdbook_version != mdbook::MDBOOK_VERSION {
        // We should probably use the `semver` crate to check compatibility
        // here...
        eprintln!(
            "Warning: The {} plugin was built against version {} of mdbook, \
             but we're being called from version {}",
            pre.name(),
            mdbook::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }

    let processed_book = pre.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}

fn handle_raw(_sub_args: &ArgMatches) -> Result<(), Error> {
    let mut markdown = String::new();
    io::stdin().read_to_string(&mut markdown)?;

    let processed = join_cjk_spacing(&markdown)?;
    let _ = io::stdout().write(processed.as_bytes());

    Ok(())
}

fn handle_supports(pre: &dyn Preprocessor, sub_args: &ArgMatches) -> ! {
    let renderer = sub_args.value_of("renderer").expect("Required argument");
    let supported = pre.supports_renderer(&renderer);

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}
