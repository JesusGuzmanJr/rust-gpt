fn main() {
    std::fs::write(
        "../target/style.css",
        rsass::compile_scss_path(
            std::path::Path::new("style/main.scss"),
            rsass::output::Format {
                style: rsass::output::Style::Compressed,
                ..Default::default()
            },
        )
        .expect("failed to compile SCSS"),
    )
    .expect("failed to write CSS");
}
