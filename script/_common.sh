set -euo pipefail

# normalize cwd to project root
cd "$(dirname "$0")/.."

[ -t 1 ] && USE_COLOR=1
use-color?() {
    [ -n "${USE_COLOR:-}" ]
}

tput() {
    use-color? && command tput "$@"
}

# override cargo invocations to configure colours appropriately
cargo() {
    CARGO_TERM_PROGRESS_WHEN=never \
        CARGO_TERM_COLOR="$(cargo-term-color)" \
        command cargo "$@"
}

cargo-term-color() {
    if use-color?; then
        echo always
    else
        echo never
    fi
}

# override tsc invocations to configure colours appropriately
tsc() {
    local args=()
    use-color? && args+=(--pretty)
    npx tsc "${args[@]}" "$@"
}

# eslint too
eslint() {
    local args=()
    use-color? && args+=(--color)
    npx eslint "${args[@]}" "$@"
}

cargo-target-dir() {
    echo "${CARGO_TARGET_DIR:-target}"
}

LOG_NUMBER=0
declare -A LOG_FILES
declare -A COMMANDS
try-command-async() {
    local log="/tmp/try-command.$$.$((LOG_NUMBER++)).log"
    truncate -s 0 "$log"

    ( "$@" ) &> "$log" &
    local child=$!

    LOG_FILES[$child]="$log"
    COMMANDS[$child]="$*"
}

try-command() {
    try-command-async "$@"
    await-commands
}

await-commands() {
    local failed=
    local pid

    while [ "${#COMMANDS[@]}" -gt 0 ]; do
        if ! wait -n -p pid; then
            error "command failed:" "${COMMANDS[$pid]}"
            cat "${LOG_FILES[$pid]}"
            failed=1
        fi

        rm -f "${LOG_FILES[$pid]}"
        unset COMMANDS[$pid]
        unset LOG_FILES[$pid]
    done

    if [ -n "$failed" ]; then
        error "some commands failed, exiting"
        exit 1
    fi
}

log() {
    echo "$(tput setaf 4 bold)---$(tput sgr0)$(tput setaf 7 bold)" "$@" "$(tput sgr0)"
}

error() {
    echo "$(tput setaf 1 bold)+++$(tput sgr0)$(tput setaf 7 bold)" "$@" "$(tput sgr0)"
}

success() {
    echo "$(tput setaf 2 bold)+++$(tput sgr0)$(tput setaf 7 bold)" "$@" "$(tput sgr0)"
}
