use anyhow::Result;

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<std::process::Output>;

    fn run_with_env(
        &self,
        program: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<std::process::Output>;
}

pub struct LocalRunner;

impl CommandRunner for LocalRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<std::process::Output> {
        let output = std::process::Command::new(program).args(args).output()?;
        Ok(output)
    }

    fn run_with_env(
        &self,
        program: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<std::process::Output> {
        let mut command = std::process::Command::new(program);
        command.args(args);
        for &(key, value) in env {
            command.env(key, value);
        }
        Ok(command.output()?)
    }
}
