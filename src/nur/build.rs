use crate::nur::config::NurConfig;
use tokio::process::Command;

pub async fn run_nur_build(dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = format!("{}/nurfile.yaml", dir);
    let contents = std::fs::read_to_string(&config_path)?;
    let config: NurConfig = serde_yaml::from_str(&contents)?;

    println!("📦 Building {}...", config.name);
    println!("📄 Raw build command from nurfile: {}", config.build.command);
    println!("📄 Expected output path from nurfile: {}", config.build.output);

    // Detectar si es Rust + WASM
    let is_rust_wasm = config.language.to_lowercase() == "rust" && config.build.output.ends_with(".wasm");

    // Forzar comando correcto para Rust WASM
    let (command, args): (String, Vec<&str>) = if is_rust_wasm {
        println!("⚙️  Rust WASM project detected. Overriding build command with cargo wasm32-wasip1 build.");
        (
            "cargo".to_string(),
            vec!["build", "--target", "wasm32-wasip1", "--release"],
        )
    } else {
        let mut parts = config.build.command.split_whitespace();
        let cmd = parts.next().unwrap_or("sh").to_string();
        (cmd, parts.collect())
    };

    println!("🚀 Running: {} {:?}", command, args);

    let output = Command::new(&command)
        .args(&args)
        .current_dir(dir)
        .output()
        .await?;

    if !output.status.success() {
        println!("❌ Build failed:\n{}", String::from_utf8_lossy(&output.stderr));
        return Err("Build failed".into());
    }

    // Validar que el output especificado exista
    let output_path = format!("{}/{}", dir, config.build.output);
    println!("🔍 Checking if build output exists at: {}", output_path);

    if !std::path::Path::new(&output_path).exists() {
        // Sugerencia inteligente para Rust/WASM
        if is_rust_wasm {
            let suggested_name = config.name.replace("-", "_"); // Coincide con nombre de crate generado por Rust
            let suggested_path = format!("{}/target/wasm32-wasip1/release/{}.wasm", dir, suggested_name);
            println!("💡 Hint: Common Rust WASM output is `{}`", suggested_path);
        }

        return Err(format!("❌ Build output file not found at: {}", output_path).into());
    }

    println!("✅ Build output found at: {}", output_path);
    Ok(())
}