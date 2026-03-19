$url = "https://github.com/Bicheka/file-express/releases/latest/download/fexpress-windows-amd64.exe"
$dest = "$HOME\AppData\Local\Microsoft\WindowsApps\fexpress.exe"

Write-Host "Downloading fexpress..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $url -OutFile $dest

Write-Host "Success! You can now use 'fexpress' in any terminal." -ForegroundColor Green
