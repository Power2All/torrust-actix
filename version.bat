@echo off
setlocal

if "%~1"=="" (
    echo Usage: %~nx0 ^<version^>
    echo   e.g. %~nx0 4.2.11
    exit /b 1
)

set "TA_VER=%~1"

echo %TA_VER%|findstr /R "^[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*$" >nul
if errorlevel 1 (
    echo Error: "%TA_VER%" is not a valid X.Y.Z version.
    exit /b 1
)

pushd "%~dp0"

echo Setting torrust-actix version to %TA_VER% ...
powershell -NoProfile -ExecutionPolicy Bypass -Command "$q=[char]34; $enc=New-Object System.Text.UTF8Encoding $false; $v=$env:TA_VER; function S($p,$pat,$rep){ if(-not(Test-Path $p)){ Write-Host ('  [skip] '+$p+' (not found)'); return }; $o=[IO.File]::ReadAllText($p); $n=[regex]::Replace($o,$pat,$rep); if($n -ne $o){ [IO.File]::WriteAllText($p,$n,$enc); Write-Host ('  [ok]   '+$p) } else { Write-Host ('  [warn] '+$p+' (no version match)') } }; S 'Cargo.toml' ('(?m)^version = '+$q+'\d+\.\d+\.\d+'+$q) ('version = '+$q+$v+$q); S 'Cargo.lock' ('(name = '+$q+'torrust-actix'+$q+'\r?\nversion = '+$q+')\d+\.\d+\.\d+') ('${1}'+$v); S 'docker\build.bat' 'torrust-actix:v\d+\.\d+\.\d+' ('torrust-actix:v'+$v); S 'docker\Dockerfile' 'tags/v\d+\.\d+\.\d+' ('tags/v'+$v)"
set "RC=%errorlevel%"

popd

if not "%RC%"=="0" (
    echo.
    echo Failed to update one or more files.
    exit /b %RC%
)

echo.
echo Done. Review the changes with: git diff
exit /b 0
