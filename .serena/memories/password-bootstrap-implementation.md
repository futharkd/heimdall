## Password Setup for Bootstrap User

**Problem:** `bootstrap user` created users without passwords. When those users ran `bootstrap netbird`, the install script internally called `sudo`, which prompted for a password the user didn't have.

**Solution:** Added interactive password prompting to `bootstrap user` command and set user password via `chpasswd` operation.

### Changes Made

1. **Core infrastructure** (`src/core/operation.rs`, `src/runner/mod.rs`):
   - Added `stdin_input: Option<String>` field to PlannedOperation struct
   - Added `run_with_stdin()` method to CommandRunner trait
   - Implemented run_with_stdin in LocalRunner (pipes stdin to child process)
   - Added run_with_stdin stub to all MockRunner implementations in tests

2. **CLI** (`src/cli/mod.rs`):
   - Added `--password: Option<String>` flag to BootstrapUserCommand

3. **Input handling** (`src/features/bootstrap/user/input.rs`):
   - Added `password: String` field to BootstrapUserConfig
   - Added interactive password prompt using `inquire::Password` if not provided via flag
   - Password prompt appears after SSH key resolution

4. **Plan building** (`src/features/bootstrap/user/plan.rs`):
   - Added `set_password` operation that runs `sudo chpasswd`
   - Password passed via stdin (format: `user:password\n`)
   - Set immediately after user creation

5. **Execution** (`src/features/bootstrap/user/execute.rs`):
   - Updated execute_plan to call `run_with_stdin()` when operation has stdin_input
   - Updated test MockRunner to implement run_with_stdin

### Password Security

- Password is **never visible** in command output (passed via stdin)
- Dry-run shows command as `sudo chpasswd` with no password embedded
- JSON output doesn't leak password
- SSH key-based login remains primary auth method
- Users can now authenticate with password for sudo

### Testing

- All 91 tests pass
- Bootstrap user tests cover idempotency, key handling, SSH hardening
- Dry-run verified: `./target/release/heimdall bootstrap user --user testadmin --key "..." --password "..." --dry-run`
- JSON output verified to not leak sensitive data

### Usage

```bash
# Interactive password prompt (TTY required)
heimdall bootstrap user --user admin --key "ssh-ed25519 ..."

# Non-interactive with flag
heimdall bootstrap user --user admin --key "ssh-ed25519 ..." --password "mypass"

# Dry-run to verify plan
heimdall bootstrap user --user admin --key "ssh-ed25519 ..." --password "mypass" --dry-run
```
