# StockWatch Local automated Windows test harness

## The only normal command

From an extracted project folder, run:

```text
START_HERE.cmd
```

That is the safe, non-installing entry point. It requires no choices and does not ask the operator to select a diagnostic.

When it finishes, open the newest `qualification-results/AUTO-*` directory and return the file named by `RETURN_THIS_FILE.txt`. It will be a single `SUPPORT_BUNDLE_*.zip`.

## Designed for weak local models / operators

A small local model should follow `scripts/AI_OPERATOR_INSTRUCTIONS.txt` literally. The master runner:

1. checks prerequisites;
2. runs the complete software/build/native qualification when prerequisites permit it;
3. automatically runs every safe focused diagnostic regardless of the main outcome;
4. collects logs and machine-readable summaries;
5. creates one support ZIP;
6. prints `FINAL_RESULT=PASS` or `FINAL_RESULT=FAIL` and `RETURN_THIS_FILE=<path>`.

The operator never needs to decide which diagnostic script matches the symptom.

## Safe versus system-changing entry points

### `START_HERE.cmd`

Safe default. It does **not**:

- install StockWatch into Windows;
- overwrite an existing installation;
- disable or reconfigure Windows Defender;
- change proxy settings;
- delete a user database;
- suspend/hibernate the computer.

### `START_FULL_WINDOWS_TEST.cmd`

Use only when installed-app qualification is explicitly desired. It builds and invokes the NSIS installed-app test. The installed-app script refuses to run if StockWatch Local is already installed, so it cannot silently replace an existing installation. By default it uninstalls the test copy after qualification.

## Automatic diagnostics

The safe master runner automatically captures:

- toolchain / Visual Studio / SDK state;
- DNS, TLS, proxy and package-registry connectivity;
- Yahoo delayed-market endpoint reachability;
- WebView2 presence;
- npm/Cargo build diagnostics;
- process startup/crash evidence;
- tray/WebView lifecycle probes;
- autostart registration probe;
- notification readiness state;
- SQLite database/backups/integrity information;
- process working-set/private memory/handles/threads samples;
- Defender/Code Integrity evidence relevant to StockWatch;
- NSIS installer metadata and hashes.

`DIAG_SLEEP_RESUME.ps1` is deliberately excluded from automatic execution because it can suspend the machine. It only runs with `-AllowSuspend` and explicit authorization.

## Output contract

Every normal run creates:

```text
qualification-results/
  AUTO-YYYYMMDD-HHMMSS/
    MASTER_SUMMARY.json
    MASTER_TRANSCRIPT.txt
    preflight.json
    native-qualification/
    diagnostics/
    support/
    SUPPORT_BUNDLE_YYYYMMDD-HHMMSS.zip
    SUPPORT_BUNDLE_SHA256.txt
    RETURN_THIS_FILE.txt
```

The support bundle uses a whitelist for machine/environment facts and does not intentionally dump the complete environment, credentials, API tokens, cookies, browser data, or arbitrary user documents.

## Focused scripts

Individual diagnostics exist only for deeper human debugging. A weak model should **not** choose among them; run `START_HERE.cmd` instead.
