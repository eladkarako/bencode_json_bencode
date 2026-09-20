::@echo off
chcp 65001 1>nul 2>nul
set "LANG=en_US.UTF-8"
set "LANGUAGE=en_US"
set "LC_CTYPE=en_US.UTF-8"
set "LC_NUMERIC=en_US.UTF-8"
set "LC_TIME=en_US.UTF-8"
set "LC_COLLATE=en_US.UTF-8"
set "LC_MONETARY=en_US.UTF-8"
set "LC_MESSAGES=en_US.UTF-8"
set "LC_PAPER=en_US.UTF-8"
set "LC_NAME=en_US.UTF-8"
set "LC_ADDRESS=en_US.UTF-8"
set "LC_TELEPHONE=en_US.UTF-8"
set "LC_MEASUREMENT=en_US.UTF-8"
set "LC_IDENTIFICATION=en_US.UTF-8"
set "LC_ALL=en_US.UTF-8"
set "TZ=UTC"

pushd "%~sdp0"

::------------------------------------------- on Windows
rustup update
cargo clean
cargo update

rustup target add   x86_64-pc-windows-msvc   i686-pc-windows-msvc
cargo build  --release  --target   x86_64-pc-windows-msvc
cargo build  --release  --target   i686-pc-windows-msvc

rustup target add   x86_64-linux-android   i686-linux-android   aarch64-linux-android   armv7-linux-androideabi
cargo build  --release  --target   x86_64-linux-android
cargo build  --release  --target   i686-linux-android
cargo build  --release  --target   aarch64-linux-android
cargo build  --release  --target   armv7-linux-androideabi

::------------------------------------------- on Linux (WSL). set +o pipefail not fail on pipe. set +o errexit no exit on error.
set "ARGS="
::------------------ don't exit if any command in a pipe fails (not just the last).
set "ARGS=%ARGS% set +o pipefail;"
::------------------ don't exit on error, continue running.
set "ARGS=%ARGS% set +o errexit;"
::------------------ print each command before executing (debug mode).
set "ARGS=%ARGS% set -o xtrace;"
::------------------ apply user/profile custom stuff and path (otherwise rustup won't be found)
set "ARGS=%ARGS% source ~/.profile;"
set "ARGS=%ARGS% source ~/.bashrc;"
::------------------ build commands (apt-get dependencies were previously installed in my local WSL)
set "ARGS=%ARGS% rustup update;"
set "ARGS=%ARGS% rustup target add   x86_64-unknown-linux-gnu   aarch64-unknown-linux-gnu   x86_64-unknown-linux-musl   aarch64-unknown-linux-musl  powerpc-unknown-linux-gnu  powerpc64-unknown-linux-gnu  powerpc64le-unknown-linux-gnu;"
set "ARGS=%ARGS% cargo build  --release  --target   x86_64-unknown-linux-gnu;"
set "ARGS=%ARGS% cargo build  --release  --target   aarch64-unknown-linux-gnu;"
set "ARGS=%ARGS% cargo build  --release  --target   x86_64-unknown-linux-musl;"
set "ARGS=%ARGS% cargo build  --release  --target   aarch64-unknown-linux-musl;"
set "ARGS=%ARGS% cargo build  --release  --target   powerpc-unknown-linux-gnu;"
set "ARGS=%ARGS% cargo build  --release  --target   powerpc64-unknown-linux-gnu;"
set "ARGS=%ARGS% cargo build  --release  --target   powerpc64le-unknown-linux-gnu;"

call wsl bash -c "%ARGS%"

pause
pause