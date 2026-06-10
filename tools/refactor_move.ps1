#requires -Version 5.1
# Refactor script: move ru_wx src/ files into domain folders, preserving git history.

Set-Location 'F:\code\ru_wx\ru_wx'

# Module-to-folder mapping. Entries whose name is a *folder* (log)
# are handled separately below; only file-backed modules go in this map.
$mapping = @{
    # core (foundation, no UI deps)
    'accelerator'      = 'core'
    'app'              = 'core'
    'busy_info'        = 'core'
    'dpi'              = 'core'
    'font'             = 'core'
    'geometry'         = 'core'
    'timer'            = 'core'
    'tooltip'          = 'core'
    'widget'           = 'core'
    # log goes into core/ as a subfolder (handled below, not here)

    # window (window types)
    'dialog'           = 'window'
    'frame'            = 'window'
    'frame_extras'     = 'window'
    'mdi'              = 'window'
    'menu'             = 'window'
    'panel'            = 'window'
    'popup_menu'       = 'window'
    'top_level_window' = 'window'

    # controls (standard widgets)
    'bitmap_button'         = 'controls'
    'button'                = 'controls'
    'check_list_box'        = 'controls'
    'checkbox'              = 'controls'
    'choice'                = 'controls'
    'colour_picker_ctrl'    = 'controls'
    'combo_box'             = 'controls'
    'date_picker_ctrl'      = 'controls'
    'gauge'                 = 'controls'
    'list_box'              = 'controls'
    'list_ctrl'             = 'controls'
    'radio_box'             = 'controls'
    'radio_button'          = 'controls'
    'slider'                = 'controls'
    'spin_button'           = 'controls'
    'spin_ctrl'             = 'controls'
    'spin_ctrl_double'      = 'controls'
    'static_bitmap'         = 'controls'
    'static_box'            = 'controls'
    'static_line'           = 'controls'
    'static_text'           = 'controls'
    'text_ctrl'             = 'controls'
    'toggle_button'         = 'controls'
    'tree_ctrl'             = 'controls'

    # containers (composite containers + sizers)
    'book'           = 'containers'
    'grid'           = 'containers'
    'grid_sizer'     = 'containers'
    'scroll_bar'     = 'containers'
    'scrolled_window' = 'containers'
    'sizer'          = 'containers'
    'splitter_window' = 'containers'
    'tab'            = 'containers'

    # chrome (menus / toolbars / status / tray)
    'aui_tool_bar' = 'chrome'
    'icon_tray'    = 'chrome'
    'status_bar'   = 'chrome'
    'tool_bar'     = 'chrome'

    # dialogs (common dialogs)
    'color_dialog'             = 'dialogs'
    'date_picker_dialog'       = 'dialogs'
    'dir_dialog'               = 'dialogs'
    'file_dialog'              = 'dialogs'
    'find_replace_dialog'      = 'dialogs'
    'font_dialog'              = 'dialogs'
    'message_box'              = 'dialogs'
    'message_dialog'           = 'dialogs'
    'progress_dialog'          = 'dialogs'
    'property_sheet_dialog'    = 'dialogs'
    'single_choice_dialog'     = 'dialogs'
    'symbol_picker_dialog'     = 'dialogs'
    'text_entry_dialog'        = 'dialogs'

    # dc (device context + drawing primitives)
    'art_provider'   = 'dc'
    'bitmap'         = 'dc'
    'bitmap_bundle'  = 'dc'
    'brush'          = 'dc'
    'dc'             = 'dc'
    'gl_canvas'      = 'dc'
    'icon'           = 'dc'
    'image'          = 'dc'
    'image_list'     = 'dc'
    'pen'            = 'dc'

    # adv (advanced widgets)
    'animation'      = 'adv'
    'animation_ctrl' = 'adv'
    'media_ctrl'     = 'adv'
    'property_grid'  = 'adv'
    'wizard'         = 'adv'

    # dnd (drag and drop)
    'drop_target'    = 'dnd'
    'ole_dnd'        = 'dnd'
}

$src = 'src'
$count = 0

foreach ($module in $mapping.Keys) {
    $dest = Join-Path $src $mapping[$module]
    if (-not (Test-Path $dest)) {
        New-Item -ItemType Directory -Force -Path $dest | Out-Null
    }
    $rs = Join-Path $src "$module.rs"
    $md = Join-Path $src "$module.md"
    if (Test-Path $rs) {
        git -C . mv $rs (Join-Path $dest "$module.rs") | Out-Null
        $count++
    } else {
        Write-Host "  MISSING: $rs"
    }
    if (Test-Path $md) {
        git -C . mv $md (Join-Path $dest "$module.md") | Out-Null
        $count++
    }
}

Write-Host ""
Write-Host "Moved $count files (.rs + .md)."

# Now move the two sub-folders (log, platform).
# log goes into core/ ; platform stays at src/platform (it's the
# platform backend dispatcher and is already at the right spot).
Write-Host ""
Write-Host "Moving src/log -> src/core/log ..."
git -C . mv src/log src/core/log | Out-Null
Write-Host "  done. src/platform/ left in place (unchanged)."
