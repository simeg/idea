#[cfg(test)]
extern crate atomic_counter;
extern crate dirs;
extern crate git2;
extern crate termcolor;
#[macro_use]
extern crate log;

use std::io;
use std::io::{Error, ErrorKind};

use crate::config_manager::ConfigManagement;
use crate::config_manager::ConfigType::{Branch, Repo};
use crate::git::GitManagement;
use crate::printer::{Print, PrintColor};
use crate::program_access::ProgramOpener;
use crate::reader::ReadInput;

pub mod config_manager;
pub mod git;
pub mod printer;
pub mod program_access;
pub mod reader;

pub struct Eureka<
    CM: ConfigManagement,
    W: Print + PrintColor,
    R: ReadInput,
    G: GitManagement,
    PO: ProgramOpener,
> {
    cm: CM,
    printer: W,
    reader: R,
    git: G,
    program_opener: PO,
}

#[derive(Debug)]
pub struct EurekaOptions {
    // Clear the stored path to the repo
    pub clear_repo: bool,

    // Clear the stored name of the branch
    pub clear_branch: bool,

    // Open idea document with $PAGER (fall back to `less`)
    pub view: bool,
}

const BRANCH_NAME_DEFAULT: &str = "master";

impl<CM, W, R, G, PO> Eureka<CM, W, R, G, PO>
where
    CM: ConfigManagement,
    W: Print + PrintColor,
    R: ReadInput,
    G: GitManagement,
    PO: ProgramOpener,
{
    pub fn new(cm: CM, printer: W, reader: R, git: G, program_opener: PO) -> Self {
        Eureka {
            cm,
            printer,
            reader,
            git,
            program_opener,
        }
    }

    pub fn run(&mut self, opts: EurekaOptions) -> io::Result<()> {
        debug!("Running with options: {:?}", &opts);
        if opts.clear_repo || opts.clear_branch {
            if opts.clear_repo {
                self.clear_repo()?;
                debug!("Cleared repo");
            }

            if opts.clear_branch {
                self.clear_branch()?;
                debug!("Cleared branch");
            }

            return Ok(());
        }

        if opts.view {
            self.open_idea_file()?;
            return Ok(());
        }

        if self.is_config_missing() {
            debug!("Config is missing");

            // If config dir is missing - create it
            if !self.cm.config_dir_exists() {
                self.cm.config_dir_create()?;
                debug!("Created config dir");
            }

            self.printer.fts_banner()?;

            // If repo path is missing - ask for it
            if self.cm.config_read(Repo).is_err() {
                self.setup_repo_path()?;
                debug!("Setup repo path successfully");
            }

            // If branch name is missing - ask for it
            if self.cm.config_read(Branch).is_err() {
                self.setup_branch_name()?;
                debug!("Setup branch name successfully");
            }

            self.printer
                .println("First time setup complete. Happy ideation!")?;
            Ok(())
        } else {
            self.ask_for_idea()
        }
    }

    fn ask_for_idea(&mut self) -> io::Result<()> {
        let mut idea_summary = String::new();

        while idea_summary.is_empty() {
            self.printer.input_header(">> Idea summary")?;
            idea_summary = self.reader.read_input()?;
        }

        let repo_path = self.cm.config_read(Repo)?;
        // We can set initialize git now as we have the repo path
        self.git
            .init(&repo_path)
            .map_err(|git_err| Error::new(ErrorKind::InvalidInput, git_err))?;

        self.program_opener
            .open_editor(&format!("{}/README.md", repo_path))
            .and(self.git_add_commit_push(idea_summary))
    }

    fn clear_repo(&self) -> io::Result<()> {
        self.cm
            .config_read(Repo)
            .and_then(|_| self.cm.config_rm(Repo))
    }

    fn clear_branch(&self) -> io::Result<()> {
        self.cm
            .config_read(Branch)
            .and_then(|_| self.cm.config_rm(Branch))
    }

    fn open_idea_file(&self) -> io::Result<()> {
        self.program_opener
            .open_pager(&format!("{}/README.md", self.cm.config_read(Repo)?))
    }

    fn git_add_commit_push(&mut self, commit_subject: String) -> io::Result<()> {
        let branch_name = self.cm.config_read(Branch)?;
        self.printer.println(&format!(
            "Adding and committing your new idea to {}..",
            &branch_name
        ))?;
        self.git
            .checkout_branch(branch_name.as_str())
            .and_then(|_| self.git.add())
            .and_then(|_| self.git.commit(commit_subject.as_str()))
            .map_err(|err| io::Error::new(ErrorKind::Other, err))?;
        self.printer.println("Added and committed!")?;

        self.printer.println("Pushing your new idea..")?;

        self.git
            .push(branch_name.as_str())
            .map_err(|err| io::Error::new(ErrorKind::Other, err))?;

        self.printer.println("Pushed!")?;

        Ok(())
    }

    fn setup_repo_path(&mut self) -> io::Result<()> {
        let mut input_repo_path = String::new();

        while input_repo_path.is_empty() {
            self.printer
                .input_header("Absolute path to your idea repo")?;
            input_repo_path = self.reader.read_input()?;
        }

        self.cm.config_write(Repo, input_repo_path)
    }

    fn setup_branch_name(&mut self) -> io::Result<()> {
        self.printer
            .input_header("Name of branch (default: master)")?;
        let mut branch_name = self.reader.read_input()?;

        // Default to "master"
        if branch_name.is_empty() {
            branch_name = BRANCH_NAME_DEFAULT.to_string();
        }

        self.cm.config_write(Branch, branch_name)
    }

    fn is_config_missing(&self) -> bool {
        self.cm.config_read(Repo).is_err() || self.cm.config_read(Branch).is_err()
    }
}
