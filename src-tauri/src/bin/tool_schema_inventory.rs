use std::path::PathBuf;

use magic_novel_lib::agent_tools::definition::ToolSchemaContext;
use magic_novel_lib::agent_tools::registry::build_tool_schema_inventory;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output_path: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => {
                let value = args.next().ok_or("--output requires a file path")?;
                output_path = Some(PathBuf::from(value));
            }
            other => return Err(format!("unsupported argument: {other}").into()),
        }
    }

    let inventory = build_tool_schema_inventory(&ToolSchemaContext::default());
    let json = format!("{}\n", serde_json::to_string_pretty(&inventory)?);

    if let Some(path) = output_path {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, json)?;
    } else {
        print!("{json}");
    }

    Ok(())
}
