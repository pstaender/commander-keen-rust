//! Options from the command line, shared by all episodes.

/// Options from the command line / config.
#[derive(Clone, Default)]
pub struct Options {
    pub debug: bool,
    pub nowait: bool,
    pub start_level: Option<i32>,
    /// --difficulty for --level (GD_EASY..GD_HARD, default normal)
    pub start_difficulty: Option<i32>,
    pub demo_only: bool,
    pub play_demo: Option<usize>,
    /// a demo file recorded with F10+D (DEMOn.CK6) to play and exit
    pub demo_file: Option<std::path::PathBuf>,
    /// --fullscreen / --windowed (otherwise the config.txt setting)
    pub fullscreen: Option<bool>,
    /// --classic-view: always the original 320 pixel wide play field
    pub classic_view: bool,
    /// --cheatmode: CHEAT OPTIONS replaces END GAME in the control panel
    pub cheatmode: bool,
    /// cheat option: finished levels can be entered again
    pub allow_reenter: bool,
}
