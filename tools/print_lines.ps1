$lines = Get-Content 'c:\Users\marco\Documents\code\test wxdragon\ru_wx\src\frame.rs'
for ($i = 895; $i -lt 980; $i++) {
    $lineNum = $i + 1
    Write-Output ("{0,5}  {1}" -f $lineNum, $lines[$i])
}
