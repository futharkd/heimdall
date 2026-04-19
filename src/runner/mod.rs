use anyhow::Result;

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<std::process::Output>;
}

pub struct LocalRunner;

impl CommandRunner for LocalRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<std::process::Output> {
        let output = std::process::Command::new(program).args(args).output()?;
        Ok(output)
    }
}
