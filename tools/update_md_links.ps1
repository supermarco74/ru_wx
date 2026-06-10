# update_md_links.ps1
# Bulk-update cross-references in .md files after the 10-folder refactor.
#
# Strategy:
#   For every .md link target `](PATH)`:
#     1. Resolve PATH to an absolute path (relative to the source file's
#        directory).  If the file exists at that absolute path, the link
#        is already correct -- leave it alone.
#     2. If it does not exist, treat the link as a pre-refactor reference
#        and look up its basename in the module-to-folder map to find the
#        new location.  Then re-compute the correct relative path.

$ErrorActionPreference = "Stop"
$srcRoot = (Resolve-Path "$PSScriptRoot/../src").Path

# module-name (no extension) -> "<folder>/<module>" relative to srcRoot.
# Sub-folders like core/log are encoded with the slash.
$map = @{
    # adv
    'animation'        = 'adv/animation'
    'animation_ctrl'   = 'adv/animation_ctrl'
    'media_ctrl'       = 'adv/media_ctrl'
    'property_grid'    = 'adv/property_grid'
    'wizard'           = 'adv/wizard'
    # chrome
    'aui_tool_bar'     = 'chrome/aui_tool_bar'
    'icon_tray'        = 'chrome/icon_tray'
    'status_bar'       = 'chrome/status_bar'
    'tool_bar'         = 'chrome/tool_bar'
    # containers
    'book'             = 'containers/book'
    'grid'             = 'containers/grid'
    'grid_sizer'       = 'containers/grid_sizer'
    'scroll_bar'       = 'containers/scroll_bar'
    'scrolled_window'  = 'containers/scrolled_window'
    'sizer'            = 'containers/sizer'
    'splitter_window'  = 'containers/splitter_window'
    'tab'              = 'containers/tab'
    # controls
    'bitmap_button'    = 'controls/bitmap_button'
    'button'           = 'controls/button'
    'check_list_box'   = 'controls/check_list_box'
    'checkbox'         = 'controls/checkbox'
    'choice'           = 'controls/choice'
    'colour_picker_ctrl' = 'controls/colour_picker_ctrl'
    'combo_box'        = 'controls/combo_box'
    'date_picker_ctrl' = 'controls/date_picker_ctrl'
    'gauge'            = 'controls/gauge'
    'list_box'         = 'controls/list_box'
    'list_ctrl'        = 'controls/list_ctrl'
    'radio_box'        = 'controls/radio_box'
    'radio_button'     = 'controls/radio_button'
    'slider'           = 'controls/slider'
    'spin_button'      = 'controls/spin_button'
    'spin_ctrl'        = 'controls/spin_ctrl'
    'spin_ctrl_double' = 'controls/spin_ctrl_double'
    'static_bitmap'    = 'controls/static_bitmap'
    'static_box'       = 'controls/static_box'
    'static_line'      = 'controls/static_line'
    'static_text'      = 'controls/static_text'
    'text_ctrl'        = 'controls/text_ctrl'
    'toggle_button'    = 'controls/toggle_button'
    'tree_ctrl'        = 'controls/tree_ctrl'
    # core
    'accelerator'      = 'core/accelerator'
    'app'              = 'core/app'
    'art_provider'     = 'dc/art_provider'             # moved to dc
    'bitmap'           = 'dc/bitmap'                   # moved to dc
    'bitmap_bundle'    = 'dc/bitmap_bundle'            # moved to dc
    'brush'            = 'dc/brush'                    # moved to dc
    'busy_info'        = 'core/busy_info'
    'dpi'              = 'core/dpi'
    'font'             = 'core/font'
    'geometry'         = 'core/geometry'
    'gl_canvas'        = 'dc/gl_canvas'                # moved to dc
    'icon'             = 'dc/icon'                     # moved to dc
    'image'            = 'dc/image'                    # moved to dc
    'image_list'       = 'dc/image_list'               # moved to dc
    'log'              = 'core/log'                    # log module itself
    'pen'              = 'dc/pen'                      # moved to dc
    'timer'            = 'core/timer'
    'tooltip'          = 'core/tooltip'
    'widget'           = 'core/widget'
    # dc
    'dc'               = 'dc/dc'
    # dialogs
    'color_dialog'           = 'dialogs/color_dialog'
    'date_picker_dialog'     = 'dialogs/date_picker_dialog'
    'dir_dialog'             = 'dialogs/dir_dialog'
    'file_dialog'            = 'dialogs/file_dialog'
    'find_replace_dialog'    = 'dialogs/find_replace_dialog'
    'font_dialog'            = 'dialogs/font_dialog'
    'message_box'            = 'dialogs/message_box'
    'message_dialog'         = 'dialogs/message_dialog'
    'progress_dialog'        = 'dialogs/progress_dialog'
    'property_sheet_dialog'  = 'dialogs/property_sheet_dialog'
    'single_choice_dialog'   = 'dialogs/single_choice_dialog'
    'symbol_picker_dialog'   = 'dialogs/symbol_picker_dialog'
    'text_entry_dialog'      = 'dialogs/text_entry_dialog'
    # dnd
    'drop_target'        = 'dnd/drop_target'
    'ole_dnd'            = 'dnd/ole_dnd'
    # window
    'dialog'             = 'window/dialog'
    'frame'              = 'window/frame'
    'frame_extras'       = 'window/frame_extras'
    'mdi'                = 'window/mdi'
    'menu'               = 'window/menu'
    'panel'              = 'window/panel'
    'popup_menu'         = 'window/popup_menu'
    'top_level_window'   = 'window/top_level_window'
    # log submodules (within core/log)
    'api_guard'         = 'core/log/api_guard'
    'formatter'         = 'core/log/formatter'
    'guards'            = 'core/log/guards'
    'levels'            = 'core/log/levels'
    'manager'           = 'core/log/manager'
    'record'            = 'core/log/record'
    'target'            = 'core/log/target'
    'win32_error'       = 'core/log/win32_error'
    # platform submodules
    'win32'             = 'platform/win32'
}

# Top-level docs that stay at src/ -- no movement.  Listed as a no-op map
# so the script doesn't flag them as "no mapping" and so the same-folder
# path stays valid.
$topLevelMap = @{
    'prelude'      = 'prelude'
    'lib'          = 'lib'
    'AI_INDEX'     = 'AI_INDEX'
    'AI_QUICKREF'  = 'AI_QUICKREF'
}

# PowerShell 5.1 (Windows PowerShell) does not have
# [System.IO.Path]::GetRelativePath, so we implement a small equivalent.
function Get-RelativePath([string]$fromDir, [string]$toAbs) {
    $fromUri = New-Object System.Uri (($fromDir -replace '\\', '/').TrimEnd('/') + '/')
    $toUri   = New-Object System.Uri (($toAbs   -replace '\\', '/'))
    $relUri  = $fromUri.MakeRelativeUri($toUri)
    $rel     = [System.Uri]::UnescapeDataString($relUri.ToString()) -replace '\\', '/'
    return $rel
}

# The regex matches any markdown link whose target is a .md file.  Optional
# #anchor captured in a separate group.
$linkPattern = '\]\(([^\s)#]+\.md)(#[^\s)]*)?\)'

$mdFiles = Get-ChildItem -Path $srcRoot -Recurse -Filter *.md
$totalReplacements = 0
$filesChanged = 0
$skipped = @()

foreach ($file in $mdFiles) {
    $text = [System.IO.File]::ReadAllText($file.FullName)
    $original = $text
    $fileReplacements = 0
    $srcDir = $file.DirectoryName

    $regex = [regex]$linkPattern
    $matches = $regex.Matches($text)

    for ($i = $matches.Count - 1; $i -ge 0; $i--) {
        $m = $matches[$i]
        $oldPath = $m.Groups[1].Value
        $anchor  = $m.Groups[2].Value   # e.g. "#section" or ""

        # Step 1: resolve the old path to an absolute filesystem path.
        if ([System.IO.Path]::IsPathRooted($oldPath)) {
            $resolvedAbs = $oldPath
        } else {
            $resolvedAbs = [System.IO.Path]::GetFullPath(
                (Join-Path $srcDir $oldPath))
        }

        # Step 2: if the link already points to an existing file, the
        # path is correct -- leave it alone.
        if (Test-Path -LiteralPath $resolvedAbs) {
            # Normalise: collapse "./xxx" and "xxx" to just "xxx" within
            # the same folder, but only if needed.  Skip if already
            # minimal.
            $newRelPath = Get-RelativePath -fromDir $srcDir -toAbs $resolvedAbs
            $oldLink = "$oldPath$anchor"
            $newLink = "$newRelPath$anchor"
            if ($newLink -ne $oldLink) {
                $text = $text.Substring(0, $m.Groups[1].Index) + $newRelPath + $text.Substring($m.Groups[1].Index + $m.Groups[1].Length)
                $fileReplacements++
            }
            continue
        }

        # Step 3: the old path doesn't resolve.  Treat it as a pre-refactor
        # reference: the basename (file stem) is the module name.
        $basename = [System.IO.Path]::GetFileNameWithoutExtension($oldPath)

        if ($map.ContainsKey($basename)) {
            $newRel = $map[$basename]
        } elseif ($topLevelMap.ContainsKey($basename)) {
            $newRel = $topLevelMap[$basename]
        } else {
            $skipped += "$($file.FullName.Substring($srcRoot.Length)): $oldPath  (no mapping for '$basename')"
            continue
        }

        $newAbs = Join-Path $srcRoot ($newRel + '.md')
        if (-not (Test-Path -LiteralPath $newAbs)) {
            $skipped += "$($file.FullName.Substring($srcRoot.Length)): $oldPath  (mapping -> $newRel not found on disk)"
            continue
        }

        $newRelPath = Get-RelativePath -fromDir $srcDir -toAbs $newAbs

        $oldLink = "$oldPath$anchor"
        $newLink = "$newRelPath$anchor"

        if ($newLink -ne $oldLink) {
            $text = $text.Substring(0, $m.Groups[1].Index) + $newRelPath + $text.Substring($m.Groups[1].Index + $m.Groups[1].Length)
            $fileReplacements++
        }
    }

    if ($text -ne $original) {
        [System.IO.File]::WriteAllText($file.FullName, $text)
        $filesChanged++
        $totalReplacements += $fileReplacements
        Write-Host "  updated: $($file.FullName.Substring($srcRoot.Length + 1))  ($fileReplacements links)"
    }
}

Write-Host ''
Write-Host '=== Summary ==='
Write-Host "Files changed:        $filesChanged"
Write-Host "Links rewritten:      $totalReplacements"
Write-Host "Skipped (no mapping): $($skipped.Count)"
if ($skipped.Count -gt 0) {
    $skipped | Select-Object -First 30 | ForEach-Object { Write-Host ('    ' + $_) }
}
