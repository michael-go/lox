use anyhow::Result;
use regex::Regex;
use test_case::test_case;

// TODO: to avoid manually listing the fixtures, can try to deduce from the directory via something like:
// - https://mattjquinn.com/2020/autogen-paramaterized-rust-tests/
// - https://crates.io/crates/test-generator
#[test_case("basic")]
#[test_case("unicode")]
#[test_case("error-synchronize")]
#[test_case("error-concat-not-string")]
#[test_case("vars")]
fn test_fixture(fname: &str) -> Result<()> {
    let lox_path = format!("tests/fixtures/{}.lox", fname);

    let output = std::process::Command::new("target/debug/rslox")
        .args([lox_path])
        .output()?;
    println!("{:?}", output);

    let expected_out_path = format!("tests/fixtures/{}.out", fname);
    let expected_out = std::fs::read_to_string(expected_out_path)?;
    let re = Regex::new(
        r"(?s)# exit code: (?P<ExitCode>\d+)\s*\n# stdout:\s*\n(?P<Stdout>.*)\n# stderr:\s*\n(?P<Stderr>.*)\n",
    )?;
    let caps = re.captures(&expected_out).unwrap();
    let exit_code = caps.name("ExitCode").unwrap().as_str().parse::<i32>()?;
    let stdout = caps.name("Stdout").unwrap().as_str();
    let stderr = caps.name("Stderr").unwrap().as_str();

    assert_eq!(exit_code, output.status.code().unwrap(), "exit code");
    assert_eq!(stdout, String::from_utf8(output.stdout)?, "stdout");
    assert_eq!(stderr, String::from_utf8(output.stderr)?, "stderr");

    Ok(())
}
