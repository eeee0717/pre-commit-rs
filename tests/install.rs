use assert_cmd::assert::OutputAssertExt;
use assert_fs::assert::PathAssert;
use assert_fs::fixture::{FileWriteStr, PathChild};
use insta::assert_snapshot;
use predicates::prelude::predicate;

use crate::common::{cmd_snapshot, TestContext};

mod common;

#[test]
fn install() -> anyhow::Result<()> {
    let context = TestContext::new();
    context.init_project();

    // Install `pre-commit` hook.
    cmd_snapshot!(context.filters(), context.install(), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    pre-commit installed at .git/hooks/pre-commit

    ----- stderr -----
    "#);

    insta::with_settings!(
        { filters => context.filters() },
        {
            assert_snapshot!(context.read(".git/hooks/pre-commit"), @r##"
            #!/usr/bin/env bash
            # File generated by pre-commit-rs: https://github.com/j178/pre-commit-rs
            # ID: 182c10f181da4464a3eec51b83331688

            ARGS=(hook-impl --hook-type=pre-commit)

            HERE="$(cd "$(dirname "$0")" && pwd)"
            ARGS+=(--hook-dir "$HERE" -- "$@")
            PRE_COMMIT="[CURRENT_EXE]"

            exec "$PRE_COMMIT" "${ARGS[@]}"

            "##);
        }
    );

    // Install `pre-commit` and `post-commit` hook.
    context
        .workdir()
        .child(".git/hooks/pre-commit")
        .write_str("#!/bin/sh\necho 'pre-commit'\n")?;

    cmd_snapshot!(context.filters(), context.install().arg("--hook-type").arg("pre-commit").arg("--hook-type").arg("post-commit"), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    Hook already exists at .git/hooks/pre-commit, move it to .git/hooks/pre-commit.legacy.
    pre-commit installed at .git/hooks/pre-commit
    pre-commit installed at .git/hooks/post-commit

    ----- stderr -----
    "#);
    insta::with_settings!(
        { filters => context.filters() },
        {
            assert_snapshot!(context.read(".git/hooks/pre-commit"), @r##"
            #!/usr/bin/env bash
            # File generated by pre-commit-rs: https://github.com/j178/pre-commit-rs
            # ID: 182c10f181da4464a3eec51b83331688

            ARGS=(hook-impl --hook-type=pre-commit)

            HERE="$(cd "$(dirname "$0")" && pwd)"
            ARGS+=(--hook-dir "$HERE" -- "$@")
            PRE_COMMIT="[CURRENT_EXE]"

            exec "$PRE_COMMIT" "${ARGS[@]}"

            "##);
        }
    );

    assert_snapshot!(context.read(".git/hooks/pre-commit.legacy"), @r##"
    #!/bin/sh
    echo 'pre-commit'
    "##);

    insta::with_settings!(
        { filters => context.filters() },
        {
            assert_snapshot!(context.read(".git/hooks/post-commit"), @r##"
            #!/usr/bin/env bash
            # File generated by pre-commit-rs: https://github.com/j178/pre-commit-rs
            # ID: 182c10f181da4464a3eec51b83331688

            ARGS=(hook-impl --hook-type=post-commit)

            HERE="$(cd "$(dirname "$0")" && pwd)"
            ARGS+=(--hook-dir "$HERE" -- "$@")
            PRE_COMMIT="[CURRENT_EXE]"

            exec "$PRE_COMMIT" "${ARGS[@]}"

            "##);
        }
    );

    // Overwrite existing hooks.
    cmd_snapshot!(context.filters(), context.install().arg("-t").arg("pre-commit").arg("--hook-type").arg("post-commit").arg("--overwrite"), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    Overwriting existing hook at .git/hooks/pre-commit
    pre-commit installed at .git/hooks/pre-commit
    Overwriting existing hook at .git/hooks/post-commit
    pre-commit installed at .git/hooks/post-commit

    ----- stderr -----
    "#);

    insta::with_settings!(
        { filters => context.filters() },
        {
            assert_snapshot!(context.read(".git/hooks/pre-commit"), @r##"
            #!/usr/bin/env bash
            # File generated by pre-commit-rs: https://github.com/j178/pre-commit-rs
            # ID: 182c10f181da4464a3eec51b83331688

            ARGS=(hook-impl --hook-type=pre-commit)

            HERE="$(cd "$(dirname "$0")" && pwd)"
            ARGS+=(--hook-dir "$HERE" -- "$@")
            PRE_COMMIT="[CURRENT_EXE]"

            exec "$PRE_COMMIT" "${ARGS[@]}"

            "##);
        }
    );
    insta::with_settings!(
        { filters => context.filters() },
        {
            assert_snapshot!(context.read(".git/hooks/post-commit"), @r##"
            #!/usr/bin/env bash
            # File generated by pre-commit-rs: https://github.com/j178/pre-commit-rs
            # ID: 182c10f181da4464a3eec51b83331688

            ARGS=(hook-impl --hook-type=post-commit)

            HERE="$(cd "$(dirname "$0")" && pwd)"
            ARGS+=(--hook-dir "$HERE" -- "$@")
            PRE_COMMIT="[CURRENT_EXE]"

            exec "$PRE_COMMIT" "${ARGS[@]}"

            "##);
        }
    );

    Ok(())
}

#[test]
fn uninstall() -> anyhow::Result<()> {
    let context = TestContext::new();

    context.init_project();

    // Hook does not exist.
    cmd_snapshot!(context.filters(), context.uninstall(), @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    .git/hooks/pre-commit does not exist, skipping.
    "#);

    // Uninstall `pre-commit` hook.
    context.install().assert().success();
    cmd_snapshot!(context.filters(), context.uninstall(), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    Uninstalled pre-commit

    ----- stderr -----
    "#);
    context
        .workdir()
        .child(".git/hooks/pre-commit")
        .assert(predicate::path::missing());

    // Hook is not managed by `pre-commit`.
    context
        .workdir()
        .child(".git/hooks/pre-commit")
        .write_str("#!/bin/sh\necho 'pre-commit'\n")?;
    cmd_snapshot!(context.filters(), context.uninstall(), @r#"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    .git/hooks/pre-commit is not managed by pre-commit, skipping.
    "#);

    // Restore previous hook.
    context.install().assert().success();
    cmd_snapshot!(context.filters(), context.uninstall(), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    Uninstalled pre-commit
    Restored previous hook to .git/hooks/pre-commit

    ----- stderr -----
    "#);

    // Uninstall multiple hooks.
    context
        .install()
        .arg("-t")
        .arg("pre-commit")
        .arg("-t")
        .arg("post-commit")
        .assert()
        .success();
    cmd_snapshot!(context.filters(), context.uninstall().arg("-t").arg("pre-commit").arg("-t").arg("post-commit"), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    Uninstalled pre-commit
    Restored previous hook to .git/hooks/pre-commit
    Uninstalled post-commit

    ----- stderr -----
    "#);

    Ok(())
}

#[test]
fn init_template_dir() {
    let context = TestContext::new();
    context.init_project();

    cmd_snapshot!(context.filters(), context.command().arg("init-templatedir").arg(".git"), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    pre-commit installed at .git/hooks/pre-commit

    ----- stderr -----
    `init.templateDir` not set to the target directory
    try `git config --global init.templateDir '.git'`?
    "#);
}
