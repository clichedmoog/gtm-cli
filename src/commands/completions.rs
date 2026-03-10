use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};
use std::io;

use crate::error::Result;

#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: Shell,
}

pub fn handle(args: CompletionsArgs) -> Result<()> {
    let mut cmd = crate::Cli::command();
    generate(args.shell, &mut cmd, "gtm", &mut io::stdout());
    Ok(())
}
