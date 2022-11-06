use anyhow::Result;
use regex::Regex;
use test_generator::test_resources;

#[test_resources("tests/fixtures/*.lox")]
// we need this wrapper as test_generator doesn't expect a Result
fn wrap_result(fname: &str) {
    test_fixture(fname).unwrap()
}

fn test_fixture(lox_path: &str) -> Result<()> {
    let expected_out_path = std::path::Path::new(lox_path).with_extension("out");
    let expected_out = std::fs::read_to_string(expected_out_path)?;
    let re = Regex::new(
        r"(?s)# exit code: (?P<ExitCode>\d+)\s*\n# stdout:\s*\n(?P<Stdout>.*)\n# stderr:\s*\n(?P<Stderr>.*)\n",
    )?;
    let caps = re.captures(&expected_out).unwrap();
    let expected_exit_code = caps.name("ExitCode").unwrap().as_str().parse::<i32>()?;
    let expected_stdout = caps.name("Stdout").unwrap().as_str();
    let expected_stderr = caps.name("Stderr").unwrap().as_str();

    let output = std::process::Command::new("target/debug/rslox")
        .args([lox_path])
        .output()?;
    println!("{:?}", output);

    assert_eq!(
        expected_exit_code,
        output.status.code().unwrap(),
        "exit code"
    );
    assert_eq!(expected_stdout, String::from_utf8(output.stdout)?, "stdout");
    assert_eq!(expected_stderr, String::from_utf8(output.stderr)?, "stderr");

    Ok(())
}
