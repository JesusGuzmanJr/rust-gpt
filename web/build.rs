use {
    oxc::{
        allocator::Allocator,
        codegen::{Codegen, CodegenOptions, CommentOptions},
        parser::Parser,
        span::SourceType,
    },
    std::{
        fs::write,
        path::{Path, PathBuf},
    },
};

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is not set"));

    println!("cargo:rerun-if-changed=style");

    write(
        out_dir.join("style.css"),
        rsass::compile_scss_path(
            Path::new("style/main.scss"),
            rsass::output::Format {
                style: rsass::output::Style::Compressed,
                ..Default::default()
            },
        )
        .expect("failed to compile SCSS"),
    )
    .expect("failed to write CSS");

    println!("cargo:rerun-if-changed=javascript");

    // Create target directories if they don't exist
    std::fs::create_dir_all(out_dir.join("javascript"))
        .expect("failed to create javascript directory");

    // Parse the JavaScript
    let source_type = SourceType::default().with_module(true);

    let mut scripts = String::from("use maud::{html, Markup, PreEscaped};\n");

    // find all js files in the src/pages directory
    for path in std::fs::read_dir("javascript")
        .expect("failed to read javascript directory")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "js"))
        .map(|entry| entry.path())
    {
        let source_text = std::fs::read_to_string(&path).expect("failed to read file");

        println!("minifying {}", path.display());

        // Create allocator for oxc
        let allocator = Allocator::default();

        let ret = Parser::new(&allocator, &source_text, source_type).parse();

        if !ret.errors.is_empty() {
            panic!("Parse errors in {}: {:?}", path.display(), ret.errors);
        }

        let program = ret.program;

        // Generate minified code
        let minified = Codegen::new()
            .with_options(CodegenOptions {
                minify: true,
                comments: CommentOptions::disabled(),
                ..Default::default()
            })
            .build(&program)
            .code;

        write(
            Path::new(&out_dir.join("javascript")).join(path.file_name().unwrap()),
            minified,
        )
        .expect("failed to write file");

        let filename = path
            .file_name()
            .expect("failed to get file name")
            .to_string_lossy();

        scripts.push_str(&format!(
            "\n/// The javascript pre-escaped inline html markup for `javascript/{filename}`\npub(crate) fn {}_script() -> Markup {{\n    html! {{\n        script {{ (PreEscaped(include_str!(\"javascript/{filename}\").to_string())) }}\n    }}\n}}\n",
            filename.replace(".js", "")
        ));
    }

    write(out_dir.join("scripts.rs"), scripts).expect("failed to write scripts.rs");
}
