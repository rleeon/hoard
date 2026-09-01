//! `hoard agents`: how an AI assistant learns to drive Hoard.
//!
//! The skill file ships **inside the binary** (`include_str!`), so an update of
//! Hoard is an update of the skill: the copy the user installed can never
//! describe a version they don't have. That is also why the file carries the
//! version it was generated from. An assistant compares it against
//! `hoard --version` and regenerates on mismatch, which costs one command.
//!
//! We deliberately don't write to the assistant's config ourselves. Those
//! directories belong to other applications (`~/.claude/`, and the equivalents
//! for other agent tools), the layout differs per tool, and dropping files into
//! them uninvited is how a sync tool ends up in a thread about malware. The
//! assistant places it, with the user watching.

use anyhow::Result;

/// The skill, with `{{VERSION}}` still unresolved.
const SKILL: &str = include_str!("../agents/SKILL.md");

/// The skill as it should land on disk.
pub fn skill() -> String {
    SKILL.replace("{{VERSION}}", env!("CARGO_PKG_VERSION"))
}

pub fn run(print_skill: bool) -> Result<()> {
    if print_skill {
        // Raw, so `hoard agents --skill > SKILL.md` is the whole install step.
        print!("{}", skill());
        return Ok(());
    }

    println!(
        "\
Hoard can be driven by an AI assistant (Claude Code, or any agent that can run
commands). It reads your saves with `hoard <command> --json` and proposes
changes; you approve them.

To set it up, paste this to your assistant:

  Run `hoard agents --skill` and save the output as a skill for yourself,
  in whichever directory you load skills from (for example
  ~/.claude/skills/hoard/SKILL.md). Then read it.

That is all. The skill ships inside Hoard, so it updates when Hoard does — your
assistant checks it is current and regenerates it on its own.

  hoard agents --skill    print the skill file
  hoard --help            every command
"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An unresolved placeholder would leave every installed copy claiming a
    /// version that doesn't exist, and the assistant's "am I current?" check
    /// would regenerate the file on every single run.
    #[test]
    fn skill_carries_the_running_version() {
        let s = skill();
        assert!(!s.contains("{{VERSION}}"), "placeholder left unresolved");
        assert!(s.contains(env!("CARGO_PKG_VERSION")));
        assert!(s.starts_with("---\n"), "frontmatter must open the file");
        assert!(s.contains("hoard agents --skill"), "no way to regenerate");
    }
}
