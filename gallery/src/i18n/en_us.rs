use super::TextKey;
use super::{ButtonKey, CheckBoxKey, ComboBoxKey, ScrollAreaKey, TextFieldKey, TooltipKey};
use super::{ColorShowcaseKey, CommonKey, HeaderKey, NavigationKey};
use super::{CursorKey, WaveformKey, WindowKey};
use super::{ListViewKey, TableKey, TreeKey};

pub(super) fn tr(key: TextKey) -> &'static str {
    match key {
        TextKey::Navigation(NavigationKey::PowerInhibit) => "Power Inhibit",
        TextKey::PowerInhibit(key) => match key {
            super::PowerInhibitKey::Description => "Request Windows to keep the display and system awake while Gallery is running. Initially off; the button calls the real Windows power request API.",
            super::PowerInhibitKey::Enable => "Enable power inhibition",
            super::PowerInhibitKey::Disable => "Disable power inhibition",
            super::PowerInhibitKey::Enabled => "ON · Windows accepted both display and system requests.",
            super::PowerInhibitKey::Disabled => "OFF · Gallery holds no display or system power request.",
            super::PowerInhibitKey::CleanupRequired => "Cleanup incomplete · a request may remain. Click Disable to retry.",
            super::PowerInhibitKey::NoError => "",
            super::PowerInhibitKey::Error => "Last operation failed: {error}",
            super::PowerInhibitKey::Instructions => "1. Set a short display-off and sleep timeout in Windows settings.\n2. Enable power inhibition here, then stop using the mouse and keyboard until the timeout has passed.\n3. Disable it and wait again to compare normal idle behavior. Restore your preferred timeouts when finished.",
            super::PowerInhibitKey::Lifecycle => "The request stays on when you switch pages, lose focus, minimize Gallery, or hide it to the tray. Return here to disable it, or fully exit Gallery. Closing to the tray keeps it on. This choice is not saved for the next launch.",
            super::PowerInhibitKey::Limits => "Only idle display-off and automatic sleep are inhibited. Manual Sleep, lid-close actions and power buttons follow Windows policy. An accepted request can still be limited by system policy.",
        },
        TextKey::Navigation(NavigationKey::Job) => "Jobs",
        TextKey::Job(key) => match key {
            super::JobKey::Description => "Real I/O, CPU and named-thread Jobs. Select a row to cancel, subscribe or remove its manager record. Receiver B joins only future items. A slow B applies backpressure; dropping it releases the producer. Progress bars follow consumed Progress items. A terminal manager state does not mean every receiver has consumed Finished. Buffer capacity: 4; at most 8 rows.",
            super::JobKey::StartIo => "Start short I/O",
            super::JobKey::StartCpu => "Start CPU",
            super::JobKey::StartThread => "Start named thread",
            super::JobKey::StartFailure => "Start failing Job",
            super::JobKey::StartLateComplete => "Start 3s uninterruptible call",
            super::JobKey::SubscribeB => "Subscribe B",
            super::JobKey::DropA => "Drop A",
            super::JobKey::DropB => "Drop B",
            super::JobKey::ToggleSlowB => "Slow / fast B",
            super::JobKey::Cancel => "Cancel selected",
            super::JobKey::Remove => "Remove manager record",
            super::JobKey::Ready => "Start a Job. Leaving the page cancels its Jobs without blocking the UI.",
            super::JobKey::Selection => "Selected #{id} · {kind} · manager {state} · B: {speed}",
            super::JobKey::Stats => "Manager: {state} · produced batches: {produced}/{total} · shared last Batch Arc: {shared}
A: items {items_a} · batches {a} · first/last sequence {first_a}/{last_a} · progress {progress_a}/{total} · {flow_a}
B: items {items_b} · batches {b} · first/last sequence {first_b}/{last_b} · progress {progress_b}/{total} · {flow_b}",
            super::JobKey::Select => "Select #{id} · {kind}",
            super::JobKey::Operation => "{action}: {result}",
            super::JobKey::Receiving => "waiting for Finished",
            super::JobKey::Detached => "no receiver",
            super::JobKey::Slow => "one item / 200ms",
            super::JobKey::Fast => "up to 8 items / frame",
            super::JobKey::Limit => "Eight rows are already present. Leave and re-enter to start a fresh page.",
        },
        TextKey::GalleryClose(key) => match key {
            super::CloseKey::Title => "Close Widget Gallery",
            super::CloseKey::Prompt => "What would you like to do?",
            super::CloseKey::Exit => "Exit the application",
            super::CloseKey::Hide => "Hide windows in the system tray",
            super::CloseKey::Confirm => "Confirm",
            super::CloseKey::Cancel => "Cancel",
            super::CloseKey::Waiting => "Waiting for background jobs to stop…",
            super::CloseKey::TimedOut => "Background shutdown timed out. Keep waiting or force exit.",
            super::CloseKey::Continuing => "Still waiting for background jobs. The original deadline is unchanged.",
            super::CloseKey::ShutdownDetails => "Jobs still executing or awaiting thread reaping: {remaining}. Global timeout: {timeout} ms. New jobs are no longer accepted.",
            super::CloseKey::ContinueWaiting => "Continue waiting",
            super::CloseKey::ForceExit => "Force exit",
            super::CloseKey::ForceWarning => "Force exit does not guarantee background cleanup, device shutdown, cache flushing or consistency of external effects.",
        },
        TextKey::Navigation(NavigationKey::Canvas) => "Canvas",
        TextKey::Canvas(key) => match key {
            super::CanvasKey::Description => {
                "Select a rectangle, then press again to move. Drag the eight square handles to resize, or the round handle to rotate (Shift: 15°). Delete removes the selection. Rectangle draws once; Esc cancels. Space + left drag pans until release; the wheel zooms at the pointer. Tools and zoom buttons affect the main view."
            }
            super::CanvasKey::Select => "Select",
            super::CanvasKey::Rectangle => "Rectangle",
            super::CanvasKey::Add => "Add sample",
            super::CanvasKey::ZoomIn => "Zoom +",
            super::CanvasKey::ZoomOut => "Zoom −",
            super::CanvasKey::Reset => "Reset main view",
            super::CanvasKey::Auxiliary => "Hide / show secondary",
            super::CanvasKey::Region => "Toggle export region",
            super::CanvasKey::Scale => "Cycle PNG scale",
            super::CanvasKey::Svg => "Export SVG",
            super::CanvasKey::Png => "Export PNG",
            super::CanvasKey::Main => "Main view",
            super::CanvasKey::Secondary => "Shared document · independent view",
            super::CanvasKey::Paths => {
                "Files: {path} / canvas.svg and canvas.png · existing files are overwritten."
            }
            super::CanvasKey::Ready => {
                "Choose SVG or PNG to export the committed shapes with a transparent background."
            }
            super::CanvasKey::Readout => {
                "Main: {zoom_a}× · center ({center_a}) · {mode}\nSecondary: {zoom_b}× · center ({center_b})\nExport: {region} · PNG {scale}×"
            }
            super::CanvasKey::AutoRegion => "content AABB",
            super::CanvasKey::FixedRegion => "region (−100, −75) … (100, 75)",
            super::CanvasKey::Saved => "Saved {path} · {size}",
            super::CanvasKey::Busy => {
                "A PNG export is already running; wait for it to finish, then retry."
            }
            super::CanvasKey::Pending => "PNG export in progress…",
            super::CanvasKey::Failed => "Export failed: {error}",
        },
        TextKey::Navigation(NavigationKey::Collapsible) => "Collapsible",
        TextKey::Collapsible(key) => match key {
            super::CollapsibleKey::Description => {
                "Only the arrow toggles the card. Edit the controls, fold and reopen: values stay. F8 / F9 toggle Basic / Advanced without moving focus first. Disable root, disable header, then restore root: the header stays disabled. Leaving this page resets the examples."
            }
            super::CollapsibleKey::Basic => "Basic settings",
            super::CollapsibleKey::BasicHint => {
                "Initially expanded · checkbox and slider stay alive"
            }
            super::CollapsibleKey::Advanced => "Advanced settings",
            super::CollapsibleKey::AdvancedHint => "Initially folded · F9 folds while editing",
            super::CollapsibleKey::Enabled => "Receive updates",
            super::CollapsibleKey::Volume => "Volume",
            super::CollapsibleKey::EditHint => "Type here, fold and reopen to retain the text.",
            super::CollapsibleKey::Expand => "Expand",
            super::CollapsibleKey::Collapse => "Collapse",
            super::CollapsibleKey::Same => "Set current again",
            super::CollapsibleKey::RootDisabled => "Toggle root disabled",
            super::CollapsibleKey::HeaderDisabled => "Toggle header disabled",
            super::CollapsibleKey::Status => {
                "Expanded: {expanded} · notifications: {events} · last: {last}\nRoot disabled: {root_disabled} · header request: {header_disabled}"
            }
        },
        TextKey::Navigation(NavigationKey::NumberInput) => "NumberInput",
        TextKey::NumberInput(key) => match key {
            super::NumberInputKey::Description => {
                "Edit a draft, then Enter or leave to commit; Esc cancels. Use arrows or hold −/+ to step. Official value and event counts change only on a real commit. Preset buttons replace drafts, including in read-only or disabled examples."
            }
            super::NumberInputKey::Signed => "i32 · 0..10 · step 3 · reachable endpoints",
            super::NumberInputKey::Unsigned => "u8 · no explicit bounds · type maximum 255",
            super::NumberInputKey::Float => {
                "f64 · step 0.1 · 2 decimals · focus for full precision"
            }
            super::NumberInputKey::ReadOnly => "Read-only · select/copy · presets still work",
            super::NumberInputKey::Disabled => "Disabled on the control · presets still work",
            super::NumberInputKey::InheritedDisabled => {
                "Disabled inherited from parent · presets still work"
            }
            super::NumberInputKey::Fixed => "Fixed range · min = max = 7 · no change events",
            super::NumberInputKey::Limits => "i64 · type limits · step i64::MAX",
            super::NumberInputKey::Narrow => {
                "50px parent · 134px control minimum · outer clip at 180px"
            }
            super::NumberInputKey::Status => {
                "Official: {value} · changes: {events} · last event: {last}"
            }
            super::NumberInputKey::Set => "Set {value}",
        },
        TextKey::Navigation(NavigationKey::Splitter) => "Splitter",
        TextKey::Splitter(key) => match key {
            super::SplitterKey::Description => {
                "Drag the separator, including outside the panes. A starts at 30% (300 / 700px in a 1008px container). Active changes clamp to the current pane limits; resizing the parent preserves the saved target and adds no value events. Esc and arrows do not control the handle."
            }
            super::SplitterKey::Status => {
                "Saved A: {target}% · actual A: {actual}% · value events: {events} · pane actions: {clicks} · rejected: {errors} · resizable: {resizable} · Disabled: {disabled}"
            }
            super::SplitterKey::PaneA => "Pane A · initial min 300 / max 800px",
            super::SplitterKey::PaneB => "Pane B · min 200 / max 700px",
            super::SplitterKey::PaneAction => "Pane action",
            super::SplitterKey::Nested => "Nested vertical · 50% · handle 12 / line 4px",
            super::SplitterKey::Top => "Top A",
            super::SplitterKey::Bottom => "Bottom B",
            super::SplitterKey::Left => "Left A",
            super::SplitterKey::Right => "Right B",
            super::SplitterKey::Low => "World 10% (clamp)",
            super::SplitterKey::High => "World 90% (clamp)",
            super::SplitterKey::Queued => "Commands 60%",
            super::SplitterKey::Same => "Set same value",
            super::SplitterKey::Invalid => "Reject NaN",
            super::SplitterKey::Resize => "Parent 1008 / 865px",
            super::SplitterKey::Constraints => "A minimum 300 / 100px",
            super::SplitterKey::Resizable => "Toggle resizable",
            super::SplitterKey::Disabled => "Toggle root Disabled",
            super::SplitterKey::Override => "Override handle colors",
            super::SplitterKey::Clear => "Restore theme colors",
            super::SplitterKey::Maximum => {
                "Finite maximum · A / B max 300px · the splitter occupies 608px of its 1000px parent"
            }
            super::SplitterKey::Minimum => {
                "External horizontal ScrollArea · 360px viewport, pane minima 300 + 400 + 8 = 708px; scroll to see B"
            }
            super::SplitterKey::Fixed => "resizable=false · pane buttons remain active",
            super::SplitterKey::DisabledExample => {
                "Root Disabled · pane buttons inherit the restriction"
            }
        },
        TextKey::Navigation(NavigationKey::ProgressBar) => "ProgressBar",
        TextKey::ProgressBar(key) => match key {
            super::ProgressBarKey::Description => {
                "Adjust progress with the external buttons to see smooth filling, backward updates and the indeterminate loop. In unknown mode, +/− changes the simulated input; switching back immediately shows that known ratio."
            }
            super::ProgressBarKey::Increase => "+10%",
            super::ProgressBarKey::Decrease => "-10%",
            super::ProgressBarKey::Reset => "Reset",
            super::ProgressBarKey::Mode => "Switch mode",
            super::ProgressBarKey::Percentage => "{percent}%",
            super::ProgressBarKey::Running => "Running, progress unknown",
            super::ProgressBarKey::Override => "Apply fill override",
            super::ProgressBarKey::Clear => "Restore theme colors",
        },
        TextKey::Navigation(NavigationKey::Tabs) => "Tabs",
        TextKey::Tabs(key) => match key {
            super::TabsKey::Description => {
                "Click a header, then use its arrow keys. Item 1 is disabled; navigation skips it and stops at either end. Edit a page, switch away and return: its text and scroll position remain. F8 / F9 select the last horizontal / vertical page without moving focus. Leaving this Gallery page resets both demos."
            }
            super::TabsKey::Horizontal => {
                "Horizontal · independent pages and automatic header scrolling"
            }
            super::TabsKey::Vertical => "Vertical · single-line long title with full tooltip",
            super::TabsKey::Title => "Workspace {index}",
            super::TabsKey::LongTitle => {
                "Advanced rendering and graphics device compatibility settings"
            }
            super::TabsKey::Page => "Page {index} · edit below, then switch and return",
            super::TabsKey::Retained => "This page scrolls independently of the tab headers.",
            super::TabsKey::First => "Select first",
            super::TabsKey::Last => "Select last",
            super::TabsKey::Reselect => "Select current again",
            super::TabsKey::ToggleRoot => "Toggle group disabled",
            super::TabsKey::ToggleHeader => "Toggle selected header disabled",
            super::TabsKey::ToggleSpace => "Toggle compact / roomy",
            super::TabsKey::Status => {
                "Selected {index} · notifications {events} · last notified {last} · group disabled: {root_disabled} · selected header disabled: {header_disabled}"
            }
            super::TabsKey::Overrides => {
                "Two short tabs fit without a scrollbar gutter. Apply group selected colors and a local override to the second header; clear to restore the theme."
            }
        },
        TextKey::Navigation(NavigationKey::Segmented) => "Segmented",
        TextKey::Segmented(super::SegmentedKey::Description) => {
            "Click a segment to focus its group. Use Left / Right horizontally or Up / Down vertically; skip disabled options and stop at the edges. Tab skips every group."
        }
        TextKey::Segmented(super::SegmentedKey::Basic) => {
            "Basic selection · independent user event count"
        }
        TextKey::Segmented(super::SegmentedKey::Natural) => "Content width · different labels",
        TextKey::Segmented(super::SegmentedKey::Equal) => "Equal width · the same labels in 360px",
        TextKey::Segmented(super::SegmentedKey::OptionDisabled) => "Disabled option · middle item",
        TextKey::Segmented(super::SegmentedKey::SelectedDisabled) => {
            "Selected and disabled · selection retained"
        }
        TextKey::Segmented(super::SegmentedKey::GroupDisabled) => {
            "Whole group disabled · programmatic selection allowed"
        }
        TextKey::Segmented(super::SegmentedKey::Vertical) => {
            "Vertical · Up / Down, middle item disabled"
        }
        TextKey::Segmented(super::SegmentedKey::Day) => "Day",
        TextKey::Segmented(super::SegmentedKey::Week) => "Week",
        TextKey::Segmented(super::SegmentedKey::Month) => "Month",
        TextKey::Segmented(super::SegmentedKey::Year) => "Year",
        TextKey::Segmented(super::SegmentedKey::Today) => "Today",
        TextKey::Segmented(super::SegmentedKey::Last7) => "Last 7 days",
        TextKey::Segmented(super::SegmentedKey::Last30) => "Last 30 days",
        TextKey::Segmented(super::SegmentedKey::Programmatic) => "Select next in disabled group",
        TextKey::Segmented(super::SegmentedKey::ToggleDisabled) => "Toggle whole group disabled",
        TextKey::Segmented(super::SegmentedKey::FocusBefore) => "Tab before group",
        TextKey::Segmented(super::SegmentedKey::FocusAfter) => "Tab after group",
        TextKey::Segmented(super::SegmentedKey::Status) => {
            "Selected index: {index} · user events: {events} · programmatic: {programmatic}"
        }
        TextKey::Segmented(super::SegmentedKey::Overrides) => {
            "Override checked backgrounds at group level and the middle option locally; other fields follow the theme."
        }
        TextKey::Navigation(NavigationKey::Slider) => "Slider",
        TextKey::Slider(super::SliderKey::Description) => {
            "Drag or use arrow keys. Values beside each control update live; vertical values increase upward."
        }
        TextKey::Slider(super::SliderKey::Continuous) => {
            "Continuous · default track 6px / thumb 20px"
        }
        TextKey::Slider(super::SliderKey::Stepped) => {
            "Step 3 · 0 → 3 → 6 → 9 → 10 / 10 → 7 → 4 → 1 → 0"
        }
        TextKey::Slider(super::SliderKey::Vertical) => "Vertical · bottom to top",
        TextKey::Slider(super::SliderKey::Custom) => "Custom · track 10px / thumb 28px",
        TextKey::Slider(super::SliderKey::Disabled) => {
            "Disabled · input blocked, position retained"
        }
        TextKey::Slider(super::SliderKey::Overrides) => {
            "Override fill, thumb and focus-ring colors through the public API; clear to restore the current theme."
        }
        TextKey::Button(ButtonKey::ToggleDescription) => {
            "Toggle Button: click or press Space / Enter to keep the selected appearance."
        }
        TextKey::Button(ButtonKey::ToggleButton) => "Toggle",
        TextKey::Button(ButtonKey::ToggleProgrammatic) => "Set / clear selected",
        TextKey::Button(ButtonKey::HoldTitle) => "Hold to confirm · 64 / 112 / 160 px",
        TextKey::Button(ButtonKey::HoldDescription) => {
            "Hold for 3 s; early release decays over 1.2 s. Enter / Space; F7 local disabled, F8 parent disabled, F9 visibility."
        }
        TextKey::Button(ButtonKey::HoldConfirm) => "Confirm",
        TextKey::Button(ButtonKey::HoldFocus) => "Focus 112px button",
        TextKey::Button(ButtonKey::HoldBlur) => "Move focus away",
        TextKey::Button(ButtonKey::HoldDisable) => "Toggle disabled (F7)",
        TextKey::Button(ButtonKey::HoldParentDisable) => "Parent disabled (F8)",
        TextKey::Button(ButtonKey::HoldVisibility) => "Hide / show (F9)",
        TextKey::Button(ButtonKey::HoldUnderlay) => "Transparent corners: underlay",
        TextKey::Button(ButtonKey::HoldOverrides) => {
            "Liquid color overrides; clear restores current theme"
        }
        TextKey::Button(ButtonKey::LoadingDescription) => {
            "Loading blocks repeated activation. Finish with the button or F1 to observe the Wait → Pointer cursor change."
        }
        TextKey::Button(ButtonKey::StartLoading) => "Start Loading",
        TextKey::Button(ButtonKey::FinishLoading) => "Finish Loading",
        TextKey::Button(ButtonKey::LinkButton) => "LinkButton",
        TextKey::Button(ButtonKey::LinkDescription) => {
            "Open an HTTP / HTTPS URL in the default browser. Hover or press to underline; disabled links cannot open."
        }
        TextKey::Button(ButtonKey::SwitchUrl) => "Switch URL",
        TextKey::Button(ButtonKey::ProgrammaticUrl) => {
            "Switch between Baidu and Google without opening a browser. Only activating the link opens its current URL."
        }
        TextKey::Button(ButtonKey::LinkOverrideDescription) => {
            "Override only the hovered link color (#7C3AED). Pressed and disabled states follow the current theme."
        }
        TextKey::Window(WindowKey::FileFilterLabel) => "{label} ({suffixes})",
        TextKey::GalleryHeader(HeaderKey::WidgetGallery) => "Widget Gallery",
        TextKey::Navigation(NavigationKey::Button) => "Button",
        TextKey::Navigation(NavigationKey::CheckBox) => "CheckBox",
        TextKey::Navigation(NavigationKey::ComboBox) => "ComboBox",
        TextKey::Navigation(NavigationKey::ScrollArea) => "ScrollArea",
        TextKey::Navigation(NavigationKey::ListView) => "ListView",
        TextKey::Navigation(NavigationKey::Tree) => "Tree",
        TextKey::Navigation(NavigationKey::Table) => "Table",
        TextKey::Navigation(NavigationKey::TextField) => "TextField",
        TextKey::Navigation(NavigationKey::Tooltip) => "Tooltip",
        TextKey::Navigation(NavigationKey::Window) => "Window",
        TextKey::Navigation(NavigationKey::Waveform) => "Waveform",
        TextKey::Navigation(NavigationKey::Cursor) => "Cursor",
        TextKey::Button(ButtonKey::RadioGroup) => "RadioGroup",
        TextKey::Common(CommonKey::ReadOnly) => "ReadOnly",
        TextKey::Common(CommonKey::Normal) => "Normal",
        TextKey::Common(CommonKey::Disabled) => "Disabled",
        TextKey::Button(ButtonKey::RadioButton) => "Radio Button",
        TextKey::Button(ButtonKey::Horizontal) => "Horizontal",
        TextKey::Button(ButtonKey::Grid) => "Grid",
        TextKey::Common(CommonKey::Text) => "Text",
        TextKey::Common(CommonKey::IconText) => "Icon + Text",
        TextKey::Button(ButtonKey::TextIcon) => "Text + Icon",
        TextKey::Common(CommonKey::Icon) => "Icon",
        TextKey::CheckBox(CheckBoxKey::ReceiveUpdates) => "Receive updates",
        TextKey::CheckBox(CheckBoxKey::Unchecked) => "Unchecked",
        TextKey::CheckBox(CheckBoxKey::Checked) => "Checked",
        TextKey::CheckBox(CheckBoxKey::Indeterminate) => "Indeterminate",
        TextKey::CheckBox(CheckBoxKey::TriState) => "Tri-State",
        TextKey::CheckBox(CheckBoxKey::TriStateCheckBox) => "Tri-State CheckBox",
        TextKey::ComboBox(ComboBoxKey::LocalSearch) => "Local search (shared data)",
        TextKey::ComboBox(ComboBoxKey::SearchPrompt) => "Search",
        TextKey::ComboBox(ComboBoxKey::NoResults) => "No matching options",
        TextKey::ComboBox(ComboBoxKey::SearchStatus) => {
            "Query: {query} | Matches: {matches} | Selected ID: {selected} | Active: {active}"
        }
        TextKey::ComboBox(ComboBoxKey::SearchInstructions) => {
            "Search fruit names in English or Chinese. Arrow keys browse; Enter confirms; Esc cancels. Reopening clears the query. The disabled search example blocks input."
        }
        TextKey::ComboBox(ComboBoxKey::Placeholder) => "Choose an option…",
        TextKey::ComboBox(ComboBoxKey::Renderers) => "Renderers",
        TextKey::ComboBox(ComboBoxKey::DataDriven) => "Data-Driven",
        TextKey::ComboBox(ComboBoxKey::InsertMoveOrEditSelectionKeepsIts) => {
            "Starts unselected with a placeholder. Insert, move or edit preserves identity. Remove selected or clear restores the placeholder without selecting another item.\nBanana starts disabled. Disabled items stay visible but cannot be selected. Program selection and clear notify on change."
        }
        TextKey::ScrollArea(ScrollAreaKey::Height80290) => "Height: 80 / 290",
        TextKey::ScrollArea(ScrollAreaKey::ToggleContent) => "Toggle content",
        TextKey::ScrollArea(ScrollAreaKey::ShowTarget) => "Show target",
        TextKey::Common(CommonKey::Basic) => "Basic",
        TextKey::Tooltip(TooltipKey::RichContent) => "Rich Content",
        TextKey::Tooltip(TooltipKey::Placement) => "Placement",
        TextKey::Tooltip(TooltipKey::BasicTooltip) => "Basic tooltip",
        TextKey::Tooltip(TooltipKey::HoverMe) => "Hover me",
        TextKey::Tooltip(TooltipKey::RichTooltip) => "Rich tooltip",
        TextKey::Tooltip(TooltipKey::IconAndMultipleLines) => "Icon and multiple lines",
        TextKey::Tooltip(TooltipKey::RichContent2) => "Rich content",
        TextKey::Tooltip(TooltipKey::DisabledControlsStillHaveTooltips) => {
            "Disabled controls still have tooltips"
        }
        TextKey::Common(CommonKey::EnableDisable) => "Enable / Disable",
        TextKey::Tree(TreeKey::SelectLastApi) => "Select last (API)",
        TextKey::Tree(TreeKey::SelectVisible) => "Select visible (Ctrl+A)",
        TextKey::Tree(TreeKey::ExpandAll) => "Expand loaded",
        TextKey::Tree(TreeKey::CollapseAll) => "Collapse all",
        TextKey::Window(WindowKey::FileDialog) => "File Dialog",
        TextKey::Window(WindowKey::ModalBlocksMainWindow) => "Modal (blocks main window)",
        TextKey::Window(WindowKey::ChooseFilesOrFoldersTypeAFile) => {
            "Choose files or folders. Type a file name to open; use File type to filter extensions. Nonmodal dialogs can stay open together."
        }
        TextKey::Window(WindowKey::Result) => "Result: ...",
        TextKey::Window(WindowKey::IndependentWindow) => "Independent Window",
        TextKey::Window(WindowKey::Theme) => "Theme",
        TextKey::Window(WindowKey::ThisIsAnIndependentWidgetryWindow) => {
            "This is an independent Widgetry window."
        }
        TextKey::Window(WindowKey::ClickMe) => "Click me",
        TextKey::Window(WindowKey::Clicked) => "Clicked!",
        TextKey::Window(WindowKey::MessageBox) => "MessageBox",
        TextKey::Window(WindowKey::ChooseAResultBelow) => "Choose a result below.",
        TextKey::Window(WindowKey::ContentButtonKeepsDialogOpen) => {
            "Content button (keeps dialog open)"
        }
        TextKey::Button(ButtonKey::ManagedText) => "Managed text",
        TextKey::Button(ButtonKey::NativeFixedText) => "Native fixed text",
        TextKey::Button(ButtonKey::TextInsideButton) => "Text inside Button",
        TextKey::Button(ButtonKey::IconInsideButton) => "Icon inside Button",
        TextKey::Button(ButtonKey::HoverPress) => "Hover / Press",
        TextKey::Common(CommonKey::Apple) => "Apple",
        TextKey::Common(CommonKey::Banana) => "Banana",
        TextKey::Common(CommonKey::Orange) => "Orange",
        TextKey::Common(CommonKey::Grape) => "Grape",
        TextKey::ComboBox(ComboBoxKey::Pear) => "Pear",
        TextKey::ComboBox(ComboBoxKey::Star) => "Star",
        TextKey::ComboBox(ComboBoxKey::Logo) => "Logo",
        TextKey::ComboBox(ComboBoxKey::Inserted) => "Inserted",
        TextKey::CheckBox(CheckBoxKey::CheckMe) => "Check me",
        TextKey::ScrollArea(ScrollAreaKey::ScrollHoverVerticalThumb) => {
            "Scroll / hover vertical thumb"
        }
        TextKey::Tooltip(TooltipKey::TooltipReadableThemeForeground) => {
            "Tooltip: readable Theme foreground"
        }
        TextKey::Tooltip(TooltipKey::HoverForTooltip) => "Hover for tooltip",
        TextKey::Common(CommonKey::InsertFirst) => "Insert first",
        TextKey::Common(CommonKey::RemoveSelected) => "Remove selected",
        TextKey::ComboBox(ComboBoxKey::MoveSelectedLast) => "Move selected → last",
        TextKey::Common(CommonKey::EditSelected) => "Edit selected",
        TextKey::ComboBox(ComboBoxKey::EnableDisableFirst) => "Enable/disable first",
        TextKey::ComboBox(ComboBoxKey::SetSelectedFirst) => "Set selected first",
        TextKey::Common(CommonKey::ClearSelection) => "Clear selection",
        TextKey::ComboBox(ComboBoxKey::ResetModel) => "Reset model",
        TextKey::ListView(ListViewKey::ResetViewOrder) => "Reset view order",
        TextKey::ListView(ListViewKey::ToggleDrag) => "Toggle drag",
        TextKey::ListView(ListViewKey::ToggleFilter) => "Toggle even filter (B)",
        TextKey::ListView(ListViewKey::ListSharedText) => "Shared Model A: text",
        TextKey::ListView(ListViewKey::ListSharedIcon) => "Shared Model B: icon + text",
        TextKey::ListView(ListViewKey::ListSharedDescription) => {
            "B reverses even labels. Drag one item before/after a row to reorder only this View; the shared Model and other View keep their order. Selection is preserved. Ctrl/Shift select; each view keeps its own selection."
        }
        TextKey::ListView(ListViewKey::SmallList) => "Small List",
        TextKey::ListView(ListViewKey::ArrowHomeEndActiveSpaceEnterSelect) => {
            "Drag enabled: move one item at a time. Click selects one; Ctrl toggles; Shift selects a range. Arrow/Home/End: active; Space/Enter: confirm."
        }
        TextKey::ListView(ListViewKey::VirtualizedLargeList) => "Virtualized Large List",
        TextKey::ListView(ListViewKey::Text10000ItemsWheelPageupPagedownJump) => {
            "Drag disabled. 10,000 items: Ctrl+A shows the full selection count. Scroll, PageUp/PageDown, edit and resize."
        }
        TextKey::ListView(ListViewKey::DisabledListview) => "Disabled ListView",
        TextKey::ListView(ListViewKey::UserInputBlockedModelSelectionAndScroll) => {
            "User input blocked; model, selection and scroll buttons remain available."
        }
        TextKey::ListView(ListViewKey::DisabledListviewitem) => "Disabled ListViewItem",
        TextKey::ListView(ListViewKey::InitiallyItems13DisabledArrowClick) => {
            "Items 1/3 disabled: excluded from clicks, Shift ranges and Ctrl+A. Arrow/click can still activate."
        }
        TextKey::ListView(ListViewKey::SetSelected) => "Set selected",
        TextKey::ListView(ListViewKey::EditVisible) => "Edit visible",
        TextKey::Common(CommonKey::RemoveFirst) => "Remove first",
        TextKey::Common(CommonKey::MoveFirstLast) => "Move first → last",
        TextKey::ListView(ListViewKey::ClearRepopulate) => "Clear + repopulate",
        TextKey::ListView(ListViewKey::Edit7500) => "Edit #7500",
        TextKey::ListView(ListViewKey::ShrinkTo3) => "Shrink to 3",
        TextKey::Common(CommonKey::ResizeViewport) => "Resize viewport",
        TextKey::ListView(ListViewKey::Scroll96Px) => "Scroll +96px",
        TextKey::ListView(ListViewKey::EnableDisable1) => "Enable/disable #1",
        TextKey::ScrollArea(ScrollAreaKey::VerticalAuto) => "Vertical Auto",
        TextKey::ScrollArea(ScrollAreaKey::WheelYPageupPagedownHomeEnd) => {
            "wheel Y / PageUp / PageDown / Home / End"
        }
        TextKey::ScrollArea(ScrollAreaKey::HorizontalAuto) => "Horizontal Auto",
        TextKey::ScrollArea(ScrollAreaKey::WheelXLeftRight) => "wheel X / Left / Right",
        TextKey::ScrollArea(ScrollAreaKey::BothAuto) => "Both Auto",
        TextKey::ScrollArea(ScrollAreaKey::Always) => "Always",
        TextKey::ScrollArea(ScrollAreaKey::Hidden) => "Hidden",
        TextKey::ScrollArea(ScrollAreaKey::AutoTransition) => "Auto transition",
        TextKey::ScrollArea(ScrollAreaKey::Scrollintoview) => "ScrollIntoView",
        TextKey::ScrollArea(ScrollAreaKey::Text300290Content) => "300 × 290 content",
        TextKey::ScrollArea(ScrollAreaKey::FitsViewport) => "Fits viewport",
        TextKey::ScrollArea(ScrollAreaKey::ScrollMeWithoutABar) => "Scroll me without a bar",
        TextKey::ScrollArea(ScrollAreaKey::Before1) => "Before 1",
        TextKey::ScrollArea(ScrollAreaKey::Before2) => "Before 2",
        TextKey::ScrollArea(ScrollAreaKey::Before3) => "Before 3",
        TextKey::ScrollArea(ScrollAreaKey::Before4) => "Before 4",
        TextKey::ScrollArea(ScrollAreaKey::Target) => "Target",
        TextKey::ScrollArea(ScrollAreaKey::AfterTarget) => "After target",
        TextKey::Table(TableKey::SelectFirstApi) => "Select first (API)",
        TextKey::Table(TableKey::ClearSelectionApi) => "Clear selection (API)",
        TextKey::Table(TableKey::UpdateFirstRow) => "Update first row",
        TextKey::Table(TableKey::TableShared) => "Shared Model",
        TextKey::Table(TableKey::TableSharedA) => "Shared Model A: 180 px columns, 34 px rows",
        TextKey::Table(TableKey::TableSharedB) => {
            "Shared Model B: 120 px columns, 28 px rows, compact padding"
        }
        TextKey::Table(TableKey::SelectAllApi) => "Select current View (API)",
        TextKey::Table(TableKey::ToggleEditing) => "Toggle editing",
        TextKey::Table(TableKey::ToggleColumnReordering) => "Enable / disable column dragging",
        TextKey::Table(TableKey::ResetColumnOrder) => "Reset View column order",
        TextKey::Table(TableKey::InsertColumn) => "Insert Model column at second position",
        TextKey::Table(TableKey::RemoveColumn) => "Remove first Model column",
        TextKey::Table(TableKey::MoveColumn) => "Move first Model column last",
        TextKey::Table(TableKey::ReplaceModel) => "Replace shared Model (both Views)",
        TextKey::Table(TableKey::ColumnReorderingDescription) => {
            "Column dragging starts disabled. Enable it, then drag a header body and release at the insertion line; release outside the headers to cancel. The rightmost 6px resizes, and arrow/filter buttons keep their own actions. Only this View changes; its first N positions remain frozen. Shared Model controls show new columns appended to manual View order, deletion repair, Model moves and row edits preserving that order, reset to current Model order, and Model replacement clearing both Views' preferences."
        }
        TextKey::Table(TableKey::ColumnOrderStatus) => {
            "\nColumn dragging: {state}\nModel columns: {model}\nView columns: {view}"
        }
        TextKey::Table(TableKey::CycleFrozen) => "Frozen columns: 0 / 1 / 2 / 99",
        TextKey::Table(TableKey::FrozenDescription) => {
            "Freeze the first N display columns per View. Horizontal scroll moves the remaining columns; both sides share vertical scroll. N stays unchanged when columns exceed the viewport width. Shared B starts with one frozen column. Resize the viewport or column widths to inspect strict clipping."
        }
        TextKey::Table(TableKey::FrozenStatus) => {
            "\nFrozen columns: configured {configured}, effective {effective}"
        }
        TextKey::Table(TableKey::CopySelection) => "Copy selection (TSV)",
        TextKey::Table(TableKey::ToggleCopyFormatter) => "Enable / disable Bool copy formatter",
        TextKey::Table(TableKey::CopyDescription) => {
            "Ctrl+C copies every selected View cell, including offscreen cells, without headers. Numbers use a column override, Bool uses yes/no, progress uses a typed formatter. Notes preserve real tabs, line breaks and quotes. Editor Ctrl+C belongs to its text selection. Disabling the Bool formatter makes copying a Bool cell fail without replacing the clipboard."
        }
        TextKey::Table(TableKey::CopyPasteHint) => {
            "Paste here or into a spreadsheet to inspect the TSV:"
        }
        TextKey::Table(TableKey::CopySucceeded) => {
            "Selection copied. Clipboard contains only data."
        }
        TextKey::Table(TableKey::CopySkipped) => {
            "Nothing copied: select data and exit editing; enable the Table."
        }
        TextKey::Table(TableKey::CopyFailed) => "Copy failed; previous clipboard preserved. ",
        TextKey::Table(TableKey::SingleLineNotes) => "Notes (Enter)",
        TextKey::Table(TableKey::MultiLineNotes) => "Notes (Ctrl+Enter)",
        TextKey::Table(TableKey::InvalidEditValue) => {
            "Enter a valid nonnegative integer, or progress between 0 and 1."
        }
        TextKey::Table(TableKey::EditRejected) => {
            "Application rejected the change. Numbers must be at most 1000; revise and resubmit."
        }
        TextKey::Table(TableKey::EditingDescription) => {
            "Enter/double-click edits a cell; Enter saves text, multiline notes use Ctrl+Enter, and changing a Boolean option saves it. Valid changes are applied automatically. Numbers above 1000 are rejected; revise and resubmit, or press Esc to discard the draft. Esc/outside click discards unsaved drafts. ReadOnly cells keep normal text and selection."
        }
        TextKey::Table(TableKey::RangeSelectionDescription) => {
            "Drag cells to select a rectangle. Shift-click extends cells, rows or columns from their own anchor; changing kind starts a new anchor. Ctrl+A or the corner selects every projected row and column, including offscreen cells; filtered rows stay excluded."
        }
        TextKey::Table(TableKey::SelectionRangeStatus) => {
            "\nSelection {kind}: rows {rows}, columns {columns}"
        }
        TextKey::Table(TableKey::SelectionEmpty) => "None",
        TextKey::Table(TableKey::SelectionRows) => "Rows",
        TextKey::Table(TableKey::SelectionColumns) => "Columns",
        TextKey::Table(TableKey::SelectionCells) => "Cells",
        TextKey::Table(TableKey::SelectionAll) => "All in View",
        TextKey::Table(TableKey::SortFilterDescription) => {
            "Header arrows request None → Ascending → Descending → None. Filter buttons toggle predicates: Name = even index, Number = divisible by 3, Bool = true, Progress >= 50%. Filters combine with AND; shared Views remain independent."
        }
        TextKey::Table(TableKey::SortFilterStatus) => "\nSort: {sort} | Active filters: {filters}",
        TextKey::Table(TableKey::ToggleProjection) => "Toggle reverse/even view",
        TextKey::Table(TableKey::ProjectionStatus) => {
            "\nView rows: {count} | Display sample: {order}"
        }
        TextKey::Table(TableKey::TableSharedDescription) => {
            "One Model and Renderer registry; independent row projections, layout, selection and scroll. B starts with even rows in reverse order; the application resubmits after data changes. Edit, insert, remove or move rows from either side."
        }
        TextKey::Table(TableKey::TableInsertRow) => "Insert first row",
        TextKey::Table(TableKey::ReplaceFirstHeader) => "Replace first header",
        TextKey::Table(TableKey::Cellvalue) => "CellValue",
        TextKey::Table(TableKey::Header) => "Header",
        TextKey::Table(TableKey::Selection) => "Selection",
        TextKey::Table(TableKey::ColumnLayout) => "Column Layout",
        TextKey::Table(TableKey::Virtualization) => "Virtualization",
        TextKey::Table(TableKey::RowNumbersAndColumnHeadersStayFixed) => {
            "Row numbers and column headers stay fixed on their own axis. Click any cell to select."
        }
        TextKey::Table(TableKey::StringNumberBoolProgressAndIconUse) => {
            "String, Number, Bool, Progress and Icon use distinct renderers. Update first row to refresh its content."
        }
        TextKey::Table(TableKey::CustomHeadersCombineAStarAndLabel) => {
            "Custom headers combine a star and label. Header and Cell content use independent renderers."
        }
        TextKey::Table(TableKey::ClickAColumnHeaderRowNumberOr) => {
            "Click a column header, row number or cell. Arrow keys move the focus border without changing selection."
        }
        TextKey::Table(TableKey::NameStays180PxWideNumberAndBool) => {
            "Name stays 180px wide; Number and Bool share remaining width at 2:1. Drag a header's right edge to resize."
        }
        TextKey::Table(TableKey::Text2000Rows40ColumnsScrollHorizontally) => {
            "2,000 rows × 40 columns. Scroll horizontally and vertically; row/column ranges and visible cell count follow the viewport."
        }
        TextKey::Table(TableKey::InitiallyDisabledEnableToSelectAndNavigate) => {
            "Initially disabled. Enable to select and navigate, then disable again: selection stays while input and hover stop."
        }
        TextKey::Tree(TreeKey::BasicTree) => "Basic Tree",
        TextKey::Tree(TreeKey::TreeSharedA) => "Shared TreeModel A",
        TextKey::Tree(TreeKey::TreeSharedB) => "Shared TreeModel B: shorter viewport",
        TextKey::Tree(TreeKey::ToggleDrag) => "Toggle drag",
        TextKey::Tree(TreeKey::TreeSharedDescription) => {
            "Independent expansion, selection, scroll and drag switch. A starts with drag on, B off. Ctrl toggles, Shift selects a range, Ctrl+A adds visible nodes. Drag one node to the upper/middle/lower third to place before/inside/after; a full outline means inside. Gray outline and forbidden cursor reject cycles or disabled targets. Shared data changes reach both views."
        }
        TextKey::Tree(TreeKey::TreeSharedFolder) => "Shared folder",
        TextKey::Tree(TreeKey::TreeSharedFile) => "File {index} (edits {edits})",
        TextKey::Tree(TreeKey::TreeInsertFile) => "Insert file",
        TextKey::Tree(TreeKey::TreeReparentSelected) => "Move file: folder / root",
        TextKey::Tree(TreeKey::ClickArrowsToExpandClickContentTo) => {
            "Arrows expand; Ctrl/Shift select; Ctrl+A adds visible nodes. Drag the upper/middle/lower third to place before/inside/after. Dragging a parent carries its subtree; multi-selection still moves one node. Cycles are rejected."
        }
        TextKey::Tree(TreeKey::EcsHeterogeneousTree) => "ECS Heterogeneous Tree",
        TextKey::Tree(TreeKey::FolderAndFileComponentsDispatchDifferentRenderers) => {
            "Folder and File Components dispatch different renderers. Business data stays on ECS nodes."
        }
        TextKey::Tree(TreeKey::LazyLoadingTree) => "Lazy Loading Tree",
        TextKey::Tree(TreeKey::ExpandRemoteFolderUnloadedLoadingLoadedCollapse) => {
            "Drag Local file before/after Remote while unloaded/loading; inside is forbidden (gray outline and forbidden cursor). Expand Remote to load once, then inside is allowed. Locked file rejects drops."
        }
        TextKey::Tree(TreeKey::VirtualizedTree) => "Virtualized Tree",
        TextKey::Tree(TreeKey::Expand10000FilesWheelPagedownScroll) => {
            "Expand 10,000 files; wheel/PageDown scroll. Only viewport rows exist."
        }
        TextKey::Tree(TreeKey::Workspace) => "Workspace",
        TextKey::Tree(TreeKey::Projects) => "Projects",
        TextKey::Tree(TreeKey::Widgetry) => "Widgetry",
        TextKey::Tree(TreeKey::Photos) => "Photos",
        TextKey::Tree(TreeKey::Notes) => "Notes",
        TextKey::Tree(TreeKey::Assets) => "Assets",
        TextKey::Tree(TreeKey::RemoteFolderUnloaded) => "Remote folder (unloaded)",
        TextKey::Tree(TreeKey::RemoteFolderLoading) => "Remote folder (loading...)",
        TextKey::Tree(TreeKey::RemoteFolderLoaded) => "Remote folder (loaded)",
        TextKey::Tree(TreeKey::Text10000Files) => "10,000 files",
        TextKey::Window(WindowKey::OpenFile) => "Open File",
        TextKey::Window(WindowKey::OpenFiles) => "Open Files",
        TextKey::Window(WindowKey::SelectFolder) => "Select Folder",
        TextKey::Window(WindowKey::SelectFolders) => "Select Folders",
        TextKey::Window(WindowKey::OpenImage) => "Open Image",
        TextKey::Window(WindowKey::SaveFile) => "Save File",
        TextKey::Window(WindowKey::Images) => "Images",
        TextKey::Window(WindowKey::TextDocuments) => "Text documents",
        TextKey::Window(WindowKey::Background1) => "Background 1",
        TextKey::Window(WindowKey::Background2) => "Background 2",
        TextKey::Window(WindowKey::Stretch) => "Stretch",
        TextKey::Window(WindowKey::Cover) => "Cover",
        TextKey::Window(WindowKey::Cover50) => "Cover 50%",
        TextKey::Window(WindowKey::Ok) => "OK",
        TextKey::Window(WindowKey::YesNo) => "Yes / No",
        TextKey::Window(WindowKey::YesNoCancel) => "Yes / No / Cancel",
        TextKey::Window(WindowKey::MessageBoxDemo) => "MessageBox Demo",
        TextKey::GalleryHeader(HeaderKey::Dark) => "Dark",
        TextKey::GalleryHeader(HeaderKey::Light) => "Light",
        TextKey::GalleryHeader(HeaderKey::PinkDream) => "Pink Dream",
        TextKey::GalleryHeader(HeaderKey::KamuriViolet) => "Kamuri Violet",
        TextKey::Tooltip(TooltipKey::Center) => "Center",
        TextKey::Tooltip(TooltipKey::Right) => "Right",
        TextKey::Tooltip(TooltipKey::Bottom) => "Bottom",
        TextKey::Waveform(WaveformKey::WaitingForWaveformPage) => "Waiting for Waveform page",
        TextKey::Waveform(WaveformKey::ModeLive) => "Live",
        TextKey::Waveform(WaveformKey::ModeReplay) => "Replay",
        TextKey::Waveform(WaveformKey::ModeDisk) => "Large local Replay",
        TextKey::Waveform(WaveformKey::DiskMissing) => {
            "Local recording missing: {path}. Generate it using the command in docs/gallery.md, then reload. The other demos remain available."
        }
        TextKey::Waveform(WaveformKey::DiskLoading) => {
            "Opening local recording: {path}. The current recording stays visible until loading completes."
        }
        TextKey::Waveform(WaveformKey::DiskReady) => {
            "Local Replay · {rate} Hz · {channels} channels · {seconds} s · {path}"
        }
        TextKey::Waveform(WaveformKey::DiskFailed) => {
            "Cannot open {path}: {error}. The current recording is retained; reload to retry."
        }
        TextKey::Waveform(WaveformKey::DiskReload) => "Reload local recording",
        TextKey::Waveform(WaveformKey::MeasurementHint) => {
            "Δt / ΔV: enable measurement, draw, move or resize the box. The waveform keeps running."
        }
        TextKey::Waveform(WaveformKey::ShowTimeGrid) => "Show time grid",
        TextKey::Waveform(WaveformKey::Algorithm) => "Reduction",
        TextKey::Waveform(WaveformKey::AlgorithmHint) => {
            "Min/Max preserves spikes; LTTB may omit extrema. Short spikes are injected into Stress Live."
        }
        TextKey::Waveform(WaveformKey::BasicLive100Hz4Channels5) => {
            "Basic Live · 100 Hz · 4 channels · channels 2/3 include gaps"
        }
        TextKey::Waveform(WaveformKey::BasicReplay8Khz4Channels5) => {
            "Basic Replay · 8 kHz · 4 channels"
        }
        TextKey::Waveform(WaveformKey::StressLive64Khz64Channels10) => {
            "Stress Live · 64 kHz · 64 channels"
        }
        TextKey::Cursor(CursorKey::CursorRealWindowsMouseValidation) => {
            "Cursor · Real Windows Mouse validation"
        }
        TextKey::Cursor(CursorKey::Pointer12Frames8Fps64PxKeep) => {
            "Pointer: 12 frames / 8 FPS / 64px. Keep the real mouse still to observe the loop."
        }
        TextKey::Cursor(CursorKey::Wait24Frames8Fps64PxThe) => {
            "Wait: 24 frames / 8 FPS / 64px. The loading ring rotates while Kamuri occasionally eats pudding."
        }
        TextKey::Cursor(CursorKey::StaticCursorSizes) => {
            "45 semantics: 34 system cursors and 11 hints. 43 static cursors: 48px; Pointer and Wait: 64px. BRP Custom Pointer does not change the OS cursor."
        }
        TextKey::Cursor(CursorKey::ReadOnlySemantic) => {
            "ReadOnly: selection and copying remain available; editing is blocked. Kamuri shows a blue-white lock; System uses Cell. Compare the disabled sample below."
        }
        TextKey::Cursor(CursorKey::DisabledOverridesHints) => {
            "Disabled overrides Pointer and ReadOnly: both samples below show NotAllowed on hover, including ReadOnly under a disabled parent."
        }
        TextKey::Cursor(CursorKey::WidgetryExtendedSemanticsKamuriSystemComparison) => {
            "Widgetry extended semantics · Kamuri / System comparison"
        }
        TextKey::Button(ButtonKey::NestedNodeContentRootButtonDisabledAffects) => {
            "Nested Node content: root Button Disabled affects WidgetryText / Icon; native fixed Text stays teal."
        }
        TextKey::Button(ButtonKey::ToggleRootButtonDisabled) => "Toggle root Button Disabled",
        TextKey::Button(ButtonKey::DisabledContainersAffectDescendantsThroughPlainNodes) => {
            "Disabled containers affect descendants through plain Nodes. After enabling the parent, only locally disabled CheckBoxes stay disabled."
        }
        TextKey::Button(ButtonKey::InheritParentDisabled) => "Inherit parent Disabled",
        TextKey::Button(ButtonKey::LocallyDisabledStaysDisabledAfterParentIs) => {
            "Locally Disabled; stays disabled after parent is enabled"
        }
        TextKey::Button(ButtonKey::ToggleContainerDisabled) => "Toggle container Disabled",
        TextKey::ColorShowcase(ColorShowcaseKey::PureTheme) => "Pure Theme",
        TextKey::ColorShowcase(ColorShowcaseKey::OverridesViaPublicApi) => {
            "Overrides via public API"
        }
        TextKey::ColorShowcase(ColorShowcaseKey::CurrentThemeParentEnabled) => {
            "Current: Theme; parent enabled"
        }
        TextKey::ColorShowcase(ColorShowcaseKey::OtherColorsFollowThemeClearOnlyAffects) => {
            "Other colors follow Theme. Clear only affects the open override instance."
        }
        TextKey::ColorShowcase(ColorShowcaseKey::ApplySampleOverrides) => "Apply sample overrides",
        TextKey::ColorShowcase(ColorShowcaseKey::ClearOverrides) => "Clear overrides",
        TextKey::ColorShowcase(ColorShowcaseKey::ToggleParentDisabled) => "Toggle parent Disabled",
        TextKey::Window(WindowKey::OpenPureTheme) => "Open pure Theme",
        TextKey::Window(WindowKey::OpenOverrideInstance) => "Open override instance",
        TextKey::Window(WindowKey::CycleTheme) => "Cycle Theme",
        TextKey::ColorShowcase(ColorShowcaseKey::ThemeSourceValuesPartStateFieldSwatch) => {
            "Theme source values: part / state / field / swatch / HEX (including alpha), rather than final widget colors."
        }
        TextKey::ColorShowcase(ColorShowcaseKey::RealWidgetsPureThemeOnTheLeft) => {
            "Real widgets: pure Theme on the left, selected overrides on the right. Apply / clear through public API; clearing restores current Theme."
        }
        TextKey::ScrollArea(ScrollAreaKey::YGutterInducesXOverflowNoCorner) => {
            "Y gutter induces X overflow; no Corner"
        }
        TextKey::ScrollArea(ScrollAreaKey::KeepBarAndGutterEvenWithoutOverflow) => {
            "Keep bar and gutter even without overflow"
        }
        TextKey::ScrollArea(ScrollAreaKey::ScrollableContentWithNoVisibleBar) => {
            "Scrollable content with no visible bar"
        }
        TextKey::ScrollArea(ScrollAreaKey::ToggleContentHeightToShowOrHide) => {
            "Toggle content height to show or hide the bar"
        }
        TextKey::ScrollArea(ScrollAreaKey::ScrollThenClickTheButtonToAlign) => {
            "Scroll, then click the button to align Target to the top"
        }
        TextKey::Button(ButtonKey::FixedColorConvenienceApi0F766ERetainedAcross) => {
            "Fixed color convenience API=#0F766E; retained across Theme and parent Disabled changes. Clear to inherit Button foreground."
        }
        TextKey::ComboBox(ComboBoxKey::OnlyFieldOpenBorder7C3AedOpenPopup) => {
            "Only field.open.border=#7C3AED; open Popup, list and indicator still use Theme."
        }
        TextKey::ListView(ListViewKey::OnlyItemSelectedBackground0F766EMovePointer) => {
            "Only item.selected.background=#0F766E; move pointer away after selection, active border stays unchanged."
        }
        TextKey::Tree(TreeKey::OnlyItemSelectedBackground0F766EMovePointer2) => {
            "Only item.selected.background=#0F766E; move pointer away after selection, expander still uses Theme."
        }
        TextKey::Table(TableKey::OnlyCellSelectedBackground0F766EMovePointer) => {
            "Only cell.selected.background=#0F766E; move pointer away after selection, focus border stays unchanged."
        }
        TextKey::Waveform(WaveformKey::OnlyNormalPalette7C3Aed0F766EB45309Be185D) => {
            "Only normal.palette=[#7C3AED,#0F766E,#B45309,#BE185D]; Disabled palette still uses Theme."
        }
        TextKey::Button(ButtonKey::OnlyHoveredBackground7C3AedRealHoverPressed) => {
            "Only hovered.background=#7C3AED; real hover, Pressed still uses Theme."
        }
        TextKey::Button(ButtonKey::OnlyOptionCheckedNormalDot0F766EMove) => {
            "Only option.checked.normal.dot=#0F766E; move pointer away after selection."
        }
        TextKey::CheckBox(CheckBoxKey::OnlyCheckedNormalMark0F766EMovePointer) => {
            "Only checked.normal.mark=#0F766E; move pointer away after Checked, Hover/Disabled still use Theme."
        }
        TextKey::TextField(TextFieldKey::OnlyEditableFocusedBorder7C3AedClickTo) => {
            "Only editable.focused.border=#7C3AED; click to focus and type."
        }
        TextKey::TextField(TextFieldKey::OnlyReadOnlyFocusedBorder7C3AedSelect) => {
            "Only read_only.focused.border=#7C3AED; select / copy allowed, editing blocked."
        }
        TextKey::ScrollArea(ScrollAreaKey::OnlyVerticalThumbHoveredBackground7C3AedHorizontal) => {
            "Only vertical.thumb.hovered.background=#7C3AED; horizontal thumb unchanged."
        }
        TextKey::Tooltip(TooltipKey::OnlyPopupNormalBackground7C3AedRealHover) => {
            "Only popup.normal.background=#7C3AED; real hover shows Tooltip, including on disabled anchors."
        }
        TextKey::Window(WindowKey::WindowOnlyFrameNormalBorder7C3AedClear) => {
            "Window: only frame.normal.border=#7C3AED; clear directly on the open instance."
        }
        TextKey::Window(WindowKey::WindowImageSameBorderOverrideCover50) => {
            "Window Image: same border override; Cover / 50% opacity survive Theme changes and clear."
        }
        TextKey::Window(WindowKey::MessageboxOnlyBodyNormalBackground0F766EApply) => {
            "MessageBox: only body.normal.background=#0F766E; apply / clear and Theme switching inside modal."
        }
        TextKey::Window(WindowKey::FiledialogOnlyEntrySelectedBackground0F766ENonmodal) => {
            "FileDialog: only entry.selected.background=#0F766E; NonModal, clear after selecting a file without reopening."
        }
        TextKey::Common(CommonKey::None) => "<none>",
        TextKey::ComboBox(ComboBoxKey::ComboStatus) => {
            "Items: {count} | selected: {selected} | index: {index} | value: {label}\nFirst item disabled: {disabled} | ValueChange: {changes}"
        }
        TextKey::ListView(ListViewKey::ListItem) => "Item {index}  (edits: {edits})",
        TextKey::ListView(ListViewKey::ListInsertedItem) => "Inserted item  (edits: {edits})",
        TextKey::ListView(ListViewKey::DragDisabled) => {
            "Drag disabled: this small list shares data with the enabled list; clicks and selection still work."
        }
        TextKey::ListView(ListViewKey::ListOrders) => {
            "\nModel: {original}\nProjection: {displayed}"
        }
        TextKey::ListView(ListViewKey::ListStatus) => {
            "Model items: {count} | Rendered rows: {rows} | Rendered range: {start}..{end}\nselected: {selected} | active: {active} | SelectionChanged: {changes}"
        }
        TextKey::Tree(TreeKey::TreeFolder) => "[Folder] {label}",
        TextKey::Tree(TreeKey::TreeFile) => "[File] {label}  ({bytes} bytes)",
        TextKey::Tree(TreeKey::TreeStatus) => {
            "Visible: {visible} | expanded: {expanded} | rows: {rows} {start}..{end} | selected ({selected_count}): {selected} | {state} | drag: {drag} | root order: {order}"
        }
        TextKey::Common(CommonKey::Enabled) => "enabled",
        TextKey::Table(TableKey::TableItem) => "Item {index}",
        TextKey::Table(TableKey::TableUpdated) => "Updated {edits}",
        TextKey::Table(TableKey::TableColumn) => "Column {index}",
        TextKey::Table(TableKey::TableUpdatedHeader) => "Updated header",
        TextKey::Table(TableKey::TableName) => "Name",
        TextKey::Table(TableKey::TableNumber) => "Number",
        TextKey::Table(TableKey::TableBool) => "Bool",
        TextKey::Table(TableKey::TableProgress) => "Progress",
        TextKey::Table(TableKey::TableStatus) => {
            "Model: {rows} rows × {columns} columns | visible cells: {cells} | rows {row_start}..{row_end}, columns {column_start}..{column_end}\nselection: {selection} | cursor: {cursor} | scroll: {scroll} | {state} | events: select {select}, resize start/change/end/cancel {start}/{change}/{end}/{cancel}"
        }
        TextKey::Tooltip(TooltipKey::TooltipCenter) => "Center placement",
        TextKey::Tooltip(TooltipKey::TooltipRight) => "Right placement",
        TextKey::Tooltip(TooltipKey::TooltipBottom) => "Bottom placement",
        TextKey::ColorShowcase(ColorShowcaseKey::ExpandThemeColors) => {
            "Expand / collapse Theme colors"
        }
        TextKey::ColorShowcase(ColorShowcaseKey::ExpandOverrides) => {
            "Expand / collapse explicit overrides"
        }
        TextKey::Button(ButtonKey::ButtonActivations) => "Button Activate: {count}",
        TextKey::ColorShowcase(ColorShowcaseKey::OverrideStatus) => {
            "Last operation: {action}; changed: {changed}"
        }
        TextKey::Cursor(CursorKey::CursorTheme) => "Current Cursor theme: {theme}",
        TextKey::Waveform(WaveformKey::WaveformStatus) => {
            "64 kHz · 64 ch · 10 s · producer {producer} / displayed {displayed} · {fps} FPS / {ms} ms · dropped {dropped}"
        }
        TextKey::Window(WindowKey::ResultWaiting) => "Result: Waiting...",
        TextKey::Window(WindowKey::ResultFailed) => "Result: Could not open dialog",
        TextKey::Window(WindowKey::ResultCancelled) => "Result: Cancelled",
        TextKey::Window(WindowKey::ResultPaths) => "Result: {count} path(s){paths}",
        TextKey::Window(WindowKey::WindowImageTitle) => "Background {index} - {mode}",
        TextKey::Window(WindowKey::OverrideWindowTitle) => "{kind} · {sample}",
        TextKey::Window(WindowKey::OverrideInstance) => "Override instance",
        TextKey::ScrollArea(ScrollAreaKey::V1) => "V 1",
        TextKey::ScrollArea(ScrollAreaKey::V2) => "V 2",
        TextKey::ScrollArea(ScrollAreaKey::V3) => "V 3",
        TextKey::ScrollArea(ScrollAreaKey::V4) => "V 4",
        TextKey::ScrollArea(ScrollAreaKey::V5) => "V 5",
        TextKey::ScrollArea(ScrollAreaKey::V6) => "V 6",
        TextKey::ScrollArea(ScrollAreaKey::H1) => "H 1",
        TextKey::ScrollArea(ScrollAreaKey::H2) => "H 2",
        TextKey::ScrollArea(ScrollAreaKey::H3) => "H 3",
        TextKey::ScrollArea(ScrollAreaKey::H4) => "H 4",
        TextKey::ScrollArea(ScrollAreaKey::H5) => "H 5",
    }
}
