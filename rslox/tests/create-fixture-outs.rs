use std::io::Write;

// meant to be run with `cargo script`

fn main() {
    for entry in std::fs::read_dir("tests/fixtures").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().to_str().unwrap().to_string();
        if path.ends_with(".lox") {
            let lox_path = path;
            let out_path = format!("{}.out", lox_path[..lox_path.len() - 4].to_string());

            println!("creating {}", out_path);

            let output = std::process::Command::new("target/debug/rslox")
                .args([&lox_path])
                .output()
                .unwrap();
            let exit_code = output.status.code().unwrap();
            let stdout = String::from_utf8(output.stdout).unwrap();
            let stderr = String::from_utf8(output.stderr).unwrap();

            let mut out = std::fs::File::create(out_path).unwrap();
            writeln!(out, "# exit code: {}", exit_code).unwrap();
            writeln!(out, "# stdout:\n{}", stdout).unwrap();
            writeln!(out, "# stderr:\n{}", stderr).unwrap();
        }
    }
}
