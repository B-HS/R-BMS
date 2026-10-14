$ErrorActionPreference = 'Stop'
$rc = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\rc.exe" | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $rc) { throw 'rc.exe was not found in the Windows SDK' }
New-Item -ItemType Directory -Force -Path target | Out-Null
$res = Join-Path (Resolve-Path target) 'rbms-icon.res'
& $rc.FullName /nologo /i assets\icon /fo $res assets\icon\rbms.rc
if ($LASTEXITCODE -ne 0) { throw "rc.exe failed with $LASTEXITCODE" }
"RBMS_WINDOWS_RESOURCE=$res" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
Write-Output "compiled $res with $($rc.FullName)"
