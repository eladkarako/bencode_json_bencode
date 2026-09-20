::@echo off
chcp 65001 1>nul 2>nul
pushd "%~dp0"

set "BINARY="
for %%I in ("%CD%") do ( 
  set "BINARY=%%~nxI"
  goto EXIT_LOOP_BINARY
) 
:EXIT_LOOP_BINARY


pushd "%CD%\target"


goto MAIN


::------------------------------------------------
:METHOD
  setlocal
  set "TARGET_NAME=%~1"
  set "FULL_PATH=%CD%\%TARGET_NAME%\release\%BINARY%"

  if exist "%FULL_PATH%.exe" (
    set "%FULL_PATH%=%FULL_PATH%.exe"
  )

  start "" /MAX /ABOVENORMAL "7z.exe" a -tzip -y -ssp -sse -ssw -mmt4 -mx9 -mm=Deflate -mem=ZipCrypto -w"%CD%" -x!"%TARGET_NAME%.zip" "%TARGET_NAME%.zip" "%FULL_PATH%"
  endlocal
  timeout /t 5
  goto :eof
::------------------------------------------------


:MAIN
for %%x in ( 
x86_64-pc-windows-msvc
i686-pc-windows-msvc
x86_64-linux-android
i686-linux-android
aarch64-linux-android
armv7-linux-androideabi
x86_64-unknown-linux-gnu
aarch64-unknown-linux-gnu
x86_64-unknown-linux-musl
aarch64-unknown-linux-musl
powerpc-unknown-linux-gnu
powerpc64-unknown-linux-gnu
powerpc64le-unknown-linux-gnu
) do ( 
  call :METHOD "%%x"
)

timeout /t 10

::-------------------------------------------------------------------------------------
:: zip packing just the binary file, of each release.
:: - zip files, named by the target's name, under '/target/'
:: - multi-process (parallel run). 4 threads, max compression. compatible zip.
:: - assumes project-name is same as binary name (often is).
:: - assumes '7z.exe' folder is in system's PATH.
::-------------------------------------------------------------------------------------
