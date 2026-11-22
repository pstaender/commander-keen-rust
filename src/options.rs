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
    /// --classic-view: always the original 320 pixel wide play field
    pub classic_view: bool,
    /// extended mode (on unless --classic): EXTENDED OPTIONS replaces END
    /// GAME in the control panel (END GAME moves into it), F2/F3 quick save/load, inventory box
    pub cheatmode: bool,
    /// extended option: finished levels can be entered again (config.txt
    /// `allowreenter`)
    pub allow_reenter: bool,
    /// extended option: dying costs no life (config.txt `unlimitedlives`)
    pub unlimited_lives: bool,
    /// extended option: firing costs no shot (config.txt `unlimitedammo`)
    pub unlimited_ammo: bool,
    /// extended option SHOW INVENTORY off: no items box at the bottom right
    /// (it is only shown in extended mode; config.txt `showinventory`)
    pub hide_inventory: bool,
    /// --no-auto-save: no AUTOSAVE.CKn while playing / resume at start
    pub no_auto_save: bool,
    /// --classic: the game as it was; implies classic_view and
    /// no_auto_save, no quick save (gamepad save/load state, the load menu's
    /// second page, F2/F3), no extended mode, no MIDI music and no volume bars
    pub classic: bool,
}
