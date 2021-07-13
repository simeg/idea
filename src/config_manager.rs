use crate::dirs::home_dir;

use std::io::{ErrorKind, Read, Write};
use std::{fs, io, path};

#[derive(Debug, Eq, PartialEq)]
pub enum ConfigType {
    Branch,
    Repo,
}

pub trait ConfigManagement {
    fn config_dir_create(&self) -> io::Result<()>;
    fn config_dir_exists(&self) -> bool;
    fn config_read(&self, config_type: ConfigType) -> io::Result<String>;
    fn config_write(&self, config_type: ConfigType, value: String) -> io::Result<()>;
    fn config_rm(&self, config_type: ConfigType) -> io::Result<()>;
}

pub struct ConfigManager;

impl Default for ConfigManager {
    fn default() -> Self {
        Self {}
    }
}

impl ConfigManagement for ConfigManager {
    fn config_dir_create(&self) -> io::Result<()> {
        self.config_dir_path().and_then(fs::create_dir_all)
    }

    fn config_dir_exists(&self) -> bool {
        self.config_dir_path().and_then(fs::metadata).is_ok()
    }

    fn config_read(&self, config_type: ConfigType) -> io::Result<String> {
        let config_path = self.config_path_for(config_type)?;
        // Make sure file exists
        fs::metadata(&config_path)?;

        let mut file = fs::File::open(&config_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        if contents.is_empty() {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                format!("File is empty: {}", &config_path),
            ));
        } else if contents.ends_with('\n') {
            contents.pop().ok_or_else(|| {
                io::Error::new(ErrorKind::Other, "Unable to remove last char from file")
            })?;
        }

        Ok(contents)
    }

    fn config_write(&self, config_type: ConfigType, value: String) -> io::Result<()> {
        let config_path = self.config_path_for(config_type)?;

        // Create file if it doesn't exist, otherwise get it
        let mut file = fs::File::create(&path::Path::new(config_path.as_str()))?;
        file.write_all(value.as_bytes())
    }

    fn config_rm(&self, config_type: ConfigType) -> io::Result<()> {
        let config_path = self.config_path_for(config_type)?;
        // Make sure file exists
        fs::metadata(&config_path)?;
        fs::remove_file(&config_path)
    }
}

impl ConfigManager {
    fn config_path_for(&self, config_type: ConfigType) -> io::Result<String> {
        let file_name = match config_type {
            // These represents files so underscore is preferred
            ConfigType::Repo => "repo_path",
            ConfigType::Branch => "branch",
        };

        self.config_dir_path()
            .map(|path| format!("{}/{}", path, file_name))
    }

    fn config_dir_path(&self) -> io::Result<String> {
        home_dir()
            .map(|home| format!("{}/{}", home.display(), ".eureka"))
            .ok_or_else(|| {
                io::Error::new(
                    ErrorKind::NotFound,
                    "Could not resolve your $HOME directory",
                )
            })
    }
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use crate::config_manager::{ConfigManagement, ConfigManager, ConfigType};
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use std::{env, fs, io, path};
    use tempfile::TempDir;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_config_manager__config_dir_path() -> TestResult {
        let cm = ConfigManager {};
        let (_config_dir, tmp_dir) = set_config_dir()?;

        let actual = cm.config_dir_path().unwrap();
        let expected = tmp_dir
            .path()
            .join(".eureka")
            .into_os_string()
            .into_string()
            .unwrap();

        env::remove_var("HOME");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_config_manager__config_dir_for__branch() -> TestResult {
        let cm = ConfigManager {};
        let (_config_dir, tmp_dir) = set_config_dir()?;

        let actual = cm.config_path_for(ConfigType::Branch).unwrap();
        let expected = tmp_dir
            .path()
            .join(".eureka/branch")
            .into_os_string()
            .into_string()
            .unwrap();

        env::remove_var("HOME");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_config_manager__config_dir_for__repo() -> TestResult {
        let cm = ConfigManager {};
        let (_config_dir, tmp_dir) = set_config_dir()?;

        let actual = cm.config_path_for(ConfigType::Repo).unwrap();
        let expected = tmp_dir
            .path()
            .join(".eureka/repo_path")
            .into_os_string()
            .into_string()
            .unwrap();

        env::remove_var("HOME");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_config_manager__config_dir_create() -> TestResult {
        let cm = ConfigManager {};
        let (_config_dir, _tmp_dir) = set_config_dir()?;

        let actual = cm.config_dir_create();

        env::remove_var("HOME");

        assert!(actual.is_ok());
        Ok(())
    }

    #[test]
    fn test_config_manager__config_dir_exists__success() -> TestResult {
        let cm = ConfigManager {};
        let (_config_dir, _tmp_dir) = set_and_create_config_dir()?;

        let config_dir_exists = cm.config_dir_exists();

        env::remove_var("HOME");

        assert!(config_dir_exists);
        Ok(())
    }

    #[test]
    fn test_config_manager__config_dir_exists__failure() -> TestResult {
        let cm = ConfigManager {};
        let (_config_dir, _tmp_dir) = set_config_dir()?;

        let config_dir_exists = cm.config_dir_exists();

        env::remove_var("HOME");

        assert!(!config_dir_exists);
        Ok(())
    }

    #[test]
    fn test_config_manager__config_read__success() -> TestResult {
        let cm = ConfigManager {};
        let (config_dir, _tmp_dir) = set_and_create_config_dir()?;
        let mut file =
            fs::File::create(&path::Path::new(&config_dir.join("repo_path").as_os_str()))?;
        file.write_all("this-repo-path-value".as_bytes())?;

        let actual = cm.config_read(ConfigType::Repo)?;
        let expected = "this-repo-path-value";

        env::remove_var("HOME");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_config_manager__config_read__file_is_empty__failure() -> TestResult {
        let cm = ConfigManager {};
        let (config_dir, _tmp_dir) = set_and_create_config_dir()?;
        // Create file but leave it empty
        let _file = fs::File::create(&path::Path::new(&config_dir.join("repo_path").as_os_str()))?;

        let actual = cm.config_read(ConfigType::Repo).map_err(|e| e.kind());
        let expected = Err(io::ErrorKind::NotFound);

        env::remove_var("HOME");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_config_manager__config_read__file_does_not_exist__failure() -> TestResult {
        let cm = ConfigManager {};
        let (_config_dir, _tmp_dir) = set_and_create_config_dir()?;

        let actual = cm.config_read(ConfigType::Repo).map_err(|e| e.kind());
        let expected = Err(io::ErrorKind::NotFound);

        env::remove_var("HOME");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_config_manager__config_write__config_file_does_not_already_exist__success() -> TestResult
    {
        let cm = ConfigManager {};
        let (config_dir, _tmp_dir) = set_and_create_config_dir()?;

        let write_result = cm.config_write(ConfigType::Repo, String::from("this-specific-value"));

        env::remove_var("HOME");

        assert!(write_result.is_ok());

        // Assert file contents
        let contents = get_file_contents(&config_dir)?;

        assert_eq!(contents, "this-specific-value");
        Ok(())
    }

    #[test]
    fn test_config_manager__config_write__config_file_already_exists__success() -> TestResult {
        let cm = ConfigManager {};
        let (config_dir, _tmp_dir) = set_and_create_config_dir()?;
        // Create file but leave it empty
        let _file = fs::File::create(&path::Path::new(&config_dir.join("repo_path").as_os_str()))?;

        let write_result = cm.config_write(ConfigType::Repo, String::from("this-specific-value"));

        env::remove_var("HOME");

        assert!(write_result.is_ok());

        // Assert file contents
        let mut file = fs::File::open(&config_dir.join("repo_path"))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        assert_eq!(contents, "this-specific-value");
        Ok(())
    }

    #[test]
    fn test_config_manager__config_rm__success() -> TestResult {
        let cm = ConfigManager {};
        let (config_dir, _tmp_dir) = set_and_create_config_dir()?;
        // Create file but leave it empty
        let _file = fs::File::create(&path::Path::new(&config_dir.join("repo_path").as_os_str()))?;

        let actual = cm.config_rm(ConfigType::Repo);

        env::remove_var("HOME");

        assert!(actual.is_ok());
        Ok(())
    }

    #[test]
    fn test_config_manager__config_rm__file_does_not_exist__failure() -> TestResult {
        let cm = ConfigManager {};
        let (_config_dir, _tmp_dir) = set_and_create_config_dir()?;

        let actual = cm.config_rm(ConfigType::Repo).map_err(|e| e.kind());
        let expected = Err(io::ErrorKind::NotFound);

        env::remove_var("HOME");

        assert_eq!(actual, expected);
        Ok(())
    }

    fn set_config_dir() -> io::Result<(PathBuf, TempDir)> {
        let tmp_dir = TempDir::new()?;
        // Create the config dir. When tmp_dir is destroyed it will be deleted
        let config_dir = tmp_dir.path().join(".eureka");

        env::set_var("HOME", tmp_dir.path());

        // tmp_dir cannot be destroyed yet, so return it
        Ok((config_dir, tmp_dir))
    }

    fn set_and_create_config_dir() -> io::Result<(PathBuf, TempDir)> {
        let (config_dir, tmp_dir) = set_config_dir()?;

        fs::create_dir_all(&config_dir)?;

        // tmp_dir cannot be destroyed yet, so return it
        Ok((config_dir, tmp_dir))
    }

    fn get_file_contents(config_dir: &PathBuf) -> io::Result<String> {
        let mut file = fs::File::open(&config_dir.join("repo_path"))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }
}
