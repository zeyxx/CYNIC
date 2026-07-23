//! Action/Effect scaffold for chat TUI.
//!
//! This keeps input handling deterministic: keys become actions, actions
//! mutate state synchronously or emit effects, effects run asynchronously.

#[derive(Debug, Clone)]
pub(crate) enum TuiAction {
    Quit,
    ClearOrQuit,
    NewSession,
    ToggleHelp,
    ToggleTaskWindow,

    FocusPrompt,
    FocusScrollback,

    ScrollScrollbackUp,
    ScrollScrollbackDown,
    ScrollScrollbackPageUp,
    ScrollScrollbackPageDown,
    ScrollScrollbackTop,
    ScrollScrollbackBottom,

    PromptInsertChar(char),
    PromptBackspace,
    PromptLeft,
    PromptRight,
    PromptHome,
    PromptEnd,
    PromptClear,
    PromptSubmit { prompt: String },
    OpenModelPicker,

    ModelPickerClose,
    ModelPickerPrev,
    ModelPickerNext,
    ModelPickerSubmit,

    PasteText(String),
}

#[derive(Debug)]
pub(crate) enum TuiEffect {
    SendPrompt { prompt: String, stream: bool },
    ApplyModelSelection { selector: String },
}
