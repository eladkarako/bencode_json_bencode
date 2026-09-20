::@echo off
chcp 65001 1>nul 2>nul
pushd "%~sdp0"
pushd "%CD%\target"


set "BINARY="
for %%F in (.\x86_64-pc-windows-msvc\release\*.exe) do ( 
  set "BINARY=%%~nF"
  goto EXIT_LOOP_BINARY
) 
:EXIT_LOOP_BINARY


goto MAIN


::------------------------------------------------
:METHOD
  setlocal
  set "TARGET=%~1"
  title %TARGET%

  set "ARGS="
  set  ARGS=%ARGS% a
  set  ARGS=%ARGS% -tzip
  ::set  ARGS=%ARGS% -x!"%TARGET%.zip"
  set  ARGS=%ARGS% -y
  set  ARGS=%ARGS% -sse
  set  ARGS=%ARGS% -ssw
  set  ARGS=%ARGS% -mmt4
  set  ARGS=%ARGS% -mx9
  set  ARGS=%ARGS% -mm=Deflate
  set  ARGS=%ARGS% -mem=ZipCrypto
  set  ARGS=%ARGS% "%TARGET%.zip"

  if exist ".\%TARGET%\release\concat.exe" ( 
    set ARGS=%ARGS% "./%TARGET%/release/concat.exe"
  ) else ( 
    set ARGS=%ARGS% "./%TARGET%/release/concat"
  ) 

  ::start "" /MAX /ABOVENORMAL /WAIT /B  "7z.exe" %ARGS%
  ::echo [INFO] EXIT-CODE: %ErrorLevel% 1>&2
  
  start "" /MAX /ABOVENORMAL  "7z.exe" %ARGS%

  endlocal
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

pause
pause
exit /b 0

::-----------------------------------------------------------------------------
:: for an easier "release" (in github for example)
::-----------------------------------------------------------------------------
:: figure-out binary name from .\target\x86_64-pc-windows-msvc\release\*.exe
:: zips the binary of each target to .\target\ folder.
:: uses target as filename for the zip.
:: assumes 7z.exe exist in system's PATH.
::-----------------------------------------------------------------------------
