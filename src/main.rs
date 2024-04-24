#![allow(clippy::ptr_arg)] // FIXME: why is clippy doing this

use errata::{FallibleExt, error};
use flate2::{Compression, GzBuilder};
use roff::Roff;
use rustdoc_types::{Crate, Id, Item, ItemEnum, Module};
use sarge::prelude::*;
use serde_json::from_str;

use std::fs;
use std::path::{Path, PathBuf};

mod gen;
mod markdown;

sarge! {
    Args,

    > "The maximum width of documentation summary lines."
    'w' max_width: usize = 80,

    > "Generate from a pre-existing JSON file."
    'j' json: bool,

    > "Remove the `output` folder before generating."
    'c' clean: bool,

    > "Features to enable. Disables default features."
    > "Defaults to all."
    #ok features: Vec<String>,

    > "The output directory."
    'o' output: String = "output".to_string(),

    > "Show this help dialogue."
    'h' help: bool,
}

#[errata::catch]
fn main() {
    let (args, files) = Args::parse().fail("failed to parse arguments");

    if args.help {
        Args::print_help();
        return;
    }

    if files.is_empty() {
        error!("expected at least 1 target file");
    }

    let output = Path::new(&args.output);
    if args.clean && output.exists() {
        fs::remove_dir_all(output).fail("failed to clean output directory");
    }

    for file in &files[1..] {
        let docs_path = if !args.json {
            let mut data_dir = dirs::data_dir().unwrap_or_else(|| "./".into());
            data_dir.push("manners");
            fs::create_dir_all(&data_dir).fail("failed to create data directory");

            rustdoc_json::Builder::default()
                .toolchain("nightly")
                .target_dir(&data_dir)
                .all_features(!args.features.as_ref().is_some_and(|v| v.is_empty()))
                .features(args.features.as_deref().unwrap_or(&[]))
                .manifest_path(file)
                .build()
                // TODO: rustdoc-json has terrible error practices
                .fail("rustdoc-json failed")
        } else {
            file.into()
        };

        let data = fs::read_to_string(docs_path).fail("failed to read JSON documentation");

        let cr = from_str(&data).fail("failed to parse JSON documentation");
        fs::create_dir_all(output).fail("failed to create output directory");

        {
            let (path, root) =
                gen::gen(&cr, &cr.root, args.max_width).fail("failed to generate manpage");
            save(root, output.join(path));
        }

        let Some(Item {
            inner: ItemEnum::Module(Module { items, .. }),
            ..
        }) = cr.index.get(&cr.root)
        else {
            unreachable!()
        };

        recurse(&cr, items, output, args.max_width);
    }
}

fn recurse(cr: &Crate, items: &[Id], output: &Path, max_width: usize) {
    for id in items {
        let Some((path, page)) = gen::gen(cr, id, max_width) else {
            // if it has no name, it's not important (an import or whatnot)
            if let Some(name) = cr
                .paths
                .get(id)
                .map(|p| p.path.join("::"))
                .or_else(|| cr.index.get(id).and_then(|i| i.name.clone()))
            {
                eprintln!("unsupported item: {name}");
            }
            continue;
        };
        eprintln!("- writing {path}");
        save(page, output.join(path));

        if let Some(Item {
            inner: ItemEnum::Module(module),
            ..
        }) = cr.index.get(id)
        {
            recurse(cr, &module.items, output, max_width);
        }
    }
}

fn save(page: Roff, mut path: PathBuf) {
    let inner_file = path.display().to_string() + ".3r";
    path.set_extension("3r.gz");
    let file = fs::File::create(&path).fail("failed to create output file");
    let mut gz = GzBuilder::new()
        .filename(inner_file)
        .write(file, Compression::default());

    page.to_writer(&mut gz)
        .fail("failed to write to output file");

    gz.finish().fail("failed to compress");
}
