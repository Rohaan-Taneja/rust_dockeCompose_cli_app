use std::process::Command;

use crate::cli_errors::CliErrors;

/**
 *
 * running a command and checking if the url is a valid github repo or not
 * command::new is a way to run command via code at runtime
 */
pub fn check_is_git_repo_url(url: &str) -> Result<bool, CliErrors> {
    // check if the url given is a valid github repo or not
    Command::new("git")
        .arg("ls-remote")
        .arg(url)
        .arg("Head")
        .output()
        .map_err(|e| {
            CliErrors::new(format!(
                "getting error while checing git repo link in docker compuse {:?}",
                e
            ))
        })?;

    Ok(true)
}
