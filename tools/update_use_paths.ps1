#requires -Version 5.1
<#
.SYNOPSIS
    Bulk-updates `use crate::xxx::yyy` paths after the 10-folder refactor.

.DESCRIPTION
    The 62 .rs files have been `git mv`-ed into 9 domain folders
    (core, window, controls, containers, chrome, dialogs, dc, adv, dnd)
    plus the log/ subfolder under core/. This script rewrites every
    `use crate::<module>::` reference so it points to the new path
    `use crate::<folder>::<module>::`.
#>

Set-Location 'F:\code\ru_wx\ru_wx'

# Module -> folder mapping. Order doesn't matter.
$mapping = [ordered]@{
    # core
    'accelerator'    = 'core'
    'app'            = 'core'
    'busy_info'      = 'core'
    'dpi'            = 'core'
    'font'           = 'core'
    'geometry'       = 'core'
    'log'            = 'core'
    'timer'          = 'core'
    'tooltip'        = 'core'
    'widget'         = 'core'
    # window
    'dialog'             = 'window'
    'frame'              = 'window'
    'frame_extras'       = 'window'
    'mdi'                = 'window'
    'menu'               = 'window'
    'panel'              = 'window'
    'popup_menu'         = 'window'
    'top_level_window'   = 'window'
    # controls
    'bitmap_button'      = 'controls'
    'button'             = 'controls'
    'check_list_box'     = 'controls'
    'checkbox'           = 'controls'
    'choice'             = 'controls'
    'colour_picker_ctrl' = 'controls'
    'combo_box'          = 'controls'
    'date_picker_ctrl'   = 'controls'
    'gauge'              = 'controls'
    'list_box'           = 'controls'
    'list_ctrl'          = 'controls'
    'radio_box'          = 'controls'
    'radio_button'       = 'controls'
    'slider'             = 'controls'
    'spin_button'        = 'controls'
    'spin_ctrl'          = 'controls'
    'spin_ctrl_double'   = 'controls'
    'static_bitmap'      = 'controls'
    'static_box'         = 'controls'
    'static_line'        = 'controls'
    'static_text'        = 'controls'
    'text_ctrl'          = 'controls'
    'toggle_button'      = 'controls'
    'tree_ctrl'          = 'controls'
    # containers
    'book'               = 'containers'
    'grid'               = 'containers'
    'grid_sizer'         = 'containers'
    'scroll_bar'         = 'containers'
    'scrolled_window'    = 'containers'
    'sizer'              = 'containers'
    'splitter_window'    = 'containers'
    'tab'                = 'containers'
    # chrome
    'aui_tool_bar'       = 'chrome'
    'icon_tray'          = 'chrome'
    'status_bar'         = 'chrome'
    'tool_bar'           = 'chrome'
    # dialogs
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
    # dc
    'art_provider'       = 'dc'
    'bitmap'             = 'dc'
    'bitmap_bundle'      = 'dc'
    'brush'              = 'dc'
    'dc'                 = 'dc'
    'gl_canvas'          = 'dc'
    'icon'               = 'dc'
    'image'              = 'dc'
    'image_list'         = 'dc'
    'pen'                = 'dc'
    # adv
    'animation'          = 'adv'
    'animation_ctrl'     = 'adv'
    'media_ctrl'         = 'adv'
    'property_grid'      = 'adv'
    'wizard'             = 'adv'
    # dnd
    'drop_target'        = 'dnd'
    'ole_dnd'            = 'dnd'
    # platform stays put
}

# Collect all .rs files under src/
$files = Get-ChildItem -Path 'src' -Recurse -Filter '*.rs' -File

$totalChanges = 0
$filesChanged = 0
$log = @()

foreach ($file in $files) {
    $text = [System.IO.File]::ReadAllText($file.FullName)
    $origText = $text
    $fileChanges = 0

    foreach ($module in $mapping.Keys) {
        $folder = $mapping[$module]
        # Match: use crate::<module>::  (case sensitive, word boundary)
        $pattern = "\b(crate::)(" + [regex]::Escape($module) + ")(::)"
        $replacement = "`$1$folder::`$2`$3"
        $newText = [regex]::Replace($text, $pattern, $replacement)
        if ($newText -ne $text) {
            $diffs = ([regex]::Matches($text, $pattern)).Count
            $fileChanges += $diffs
            $text = $newText
        }
    }

    if ($text -ne $origText) {
        [System.IO.File]::WriteAllText($file.FullName, $text, [System.Text.Encoding]::UTF8)
        $relPath = $file.FullName -replace '.*\\src\\', 'src\'
        $log += "  $relPath : $fileChanges replacement(s)"
        $totalChanges += $fileChanges
        $filesChanged++
    }
}

Write-Host ""
Write-Host "=== Summary ==="
Write-Host "Files changed: $filesChanged"
Write-Host "Total replacements: $totalChanges"
Write-Host ""
$log | ForEach-Object { Write-Host $_ }
Write-Host ""
Write-Host "DONE."
