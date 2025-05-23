// Copyright 2022 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::Path;
use std::path::PathBuf;

use crate::common::CommandOutput;
use crate::common::TestEnvironment;

#[test]
fn test_squash() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "a"])
        .success();
    std::fs::write(repo_path.join("file1"), "a\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "b"])
        .success();
    std::fs::write(repo_path.join("file1"), "b\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "c"])
        .success();
    std::fs::write(repo_path.join("file1"), "c\n").unwrap();
    // Test the setup
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  382c9bad7d42 c
    ○  d5d59175b481 b
    ○  184ddbcce5a9 a
    ◆  000000000000 (empty)
    [EOF]
    ");

    // Squashes the working copy into the parent by default
    let output = test_env.run_jj_in(&repo_path, ["squash"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: vruxwmqv f7bb78d8 (empty) (no description set)
    Parent commit (@-)      : kkmpptxz 59f44460 b c | (no description set)
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  f7bb78d8da62 (empty)
    ○  59f4446070a0 b c
    ○  184ddbcce5a9 a
    ◆  000000000000 (empty)
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1"]);
    insta::assert_snapshot!(output, @r"
    c
    [EOF]
    ");

    // Can squash a given commit into its parent
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    let output = test_env.run_jj_in(&repo_path, ["squash", "-r", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 descendant commits
    Working copy  (@) now at: mzvwutvl 1d70f50a c | (no description set)
    Parent commit (@-)      : qpvuntsm 9146bcc8 a b | (no description set)
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  1d70f50afa6d c
    ○  9146bcc8d996 a b
    ◆  000000000000 (empty)
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1", "-r", "b"]);
    insta::assert_snapshot!(output, @r"
    b
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1"]);
    insta::assert_snapshot!(output, @r"
    c
    [EOF]
    ");

    // Cannot squash a merge commit (because it's unclear which parent it should go
    // into)
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    test_env.run_jj_in(&repo_path, ["edit", "b"]).success();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "d"])
        .success();
    std::fs::write(repo_path.join("file2"), "d\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new", "c", "d"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "e"])
        .success();
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @    41219719ab5f e (empty)
    ├─╮
    │ ○  f86e2b3af3e3 d
    ○ │  382c9bad7d42 c
    ├─╯
    ○  d5d59175b481 b
    ○  184ddbcce5a9 a
    ◆  000000000000 (empty)
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["squash"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Cannot squash merge commits without a specified destination
    Hint: Use `--into` to specify which parent to squash into
    [EOF]
    [exit status: 1]
    ");

    // Can squash into a merge commit
    test_env.run_jj_in(&repo_path, ["new", "e"]).success();
    std::fs::write(repo_path.join("file1"), "e\n").unwrap();
    let output = test_env.run_jj_in(&repo_path, ["squash"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: xlzxqlsl b50b843d (empty) (no description set)
    Parent commit (@-)      : nmzmmopx 338cbc05 e | (no description set)
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  b50b843d8555 (empty)
    ○    338cbc05e4e6 e
    ├─╮
    │ ○  f86e2b3af3e3 d
    ○ │  382c9bad7d42 c
    ├─╯
    ○  d5d59175b481 b
    ○  184ddbcce5a9 a
    ◆  000000000000 (empty)
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1", "-r", "e"]);
    insta::assert_snapshot!(output, @r"
    e
    [EOF]
    ");
}

#[test]
fn test_squash_partial() {
    let mut test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "a"])
        .success();
    std::fs::write(repo_path.join("file1"), "a\n").unwrap();
    std::fs::write(repo_path.join("file2"), "a\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "b"])
        .success();
    std::fs::write(repo_path.join("file1"), "b\n").unwrap();
    std::fs::write(repo_path.join("file2"), "b\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "c"])
        .success();
    std::fs::write(repo_path.join("file1"), "c\n").unwrap();
    std::fs::write(repo_path.join("file2"), "c\n").unwrap();
    // Test the setup
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  a0b1a272ebc4 c
    ○  d117da276a0f b
    ○  54d3c1c0e9fd a
    ◆  000000000000 (empty)
    [EOF]
    ");

    // If we don't make any changes in the diff-editor, the whole change is moved
    // into the parent
    let edit_script = test_env.set_up_fake_diff_editor();
    std::fs::write(&edit_script, "dump JJ-INSTRUCTIONS instrs").unwrap();
    let output = test_env.run_jj_in(&repo_path, ["squash", "-r", "b", "-i"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 descendant commits
    Working copy  (@) now at: mzvwutvl 3c633226 c | (no description set)
    Parent commit (@-)      : qpvuntsm 38ffd8b9 a b | (no description set)
    [EOF]
    ");

    insta::assert_snapshot!(
        std::fs::read_to_string(test_env.env_root().join("instrs")).unwrap(), @r"
    You are moving changes from: kkmpptxz d117da27 b | (no description set)
    into commit: qpvuntsm 54d3c1c0 a | (no description set)

    The left side of the diff shows the contents of the parent commit. The
    right side initially shows the contents of the commit you're moving
    changes from.

    Adjust the right side until the diff shows the changes you want to move
    to the destination. If you don't make any changes, then all the changes
    from the source will be moved into the destination.
    ");

    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  3c6332267ea8 c
    ○  38ffd8b98578 a b
    ◆  000000000000 (empty)
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1", "-r", "a"]);
    insta::assert_snapshot!(output, @r"
    b
    [EOF]
    ");

    // Can squash only some changes in interactive mode
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    std::fs::write(&edit_script, "reset file1").unwrap();
    let output = test_env.run_jj_in(&repo_path, ["squash", "-r", "b", "-i"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 descendant commits
    Working copy  (@) now at: mzvwutvl 57c3cf20 c | (no description set)
    Parent commit (@-)      : kkmpptxz c4925e01 b | (no description set)
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  57c3cf20d0b1 c
    ○  c4925e01d298 b
    ○  1fc159063ed3 a
    ◆  000000000000 (empty)
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1", "-r", "a"]);
    insta::assert_snapshot!(output, @r"
    a
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file2", "-r", "a"]);
    insta::assert_snapshot!(output, @r"
    b
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1", "-r", "b"]);
    insta::assert_snapshot!(output, @r"
    b
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file2", "-r", "b"]);
    insta::assert_snapshot!(output, @r"
    b
    [EOF]
    ");

    // Can squash only some changes in non-interactive mode
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    // Clear the script so we know it won't be used even without -i
    std::fs::write(&edit_script, "").unwrap();
    let output = test_env.run_jj_in(&repo_path, ["squash", "-r", "b", "file2"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 descendant commits
    Working copy  (@) now at: mzvwutvl 64d7ad7c c | (no description set)
    Parent commit (@-)      : kkmpptxz 60a26452 b | (no description set)
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  64d7ad7c43c1 c
    ○  60a264527aee b
    ○  7314692d32e3 a
    ◆  000000000000 (empty)
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1", "-r", "a"]);
    insta::assert_snapshot!(output, @r"
    a
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file2", "-r", "a"]);
    insta::assert_snapshot!(output, @r"
    b
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1", "-r", "b"]);
    insta::assert_snapshot!(output, @r"
    b
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file2", "-r", "b"]);
    insta::assert_snapshot!(output, @r"
    b
    [EOF]
    ");

    // If we specify only a non-existent file, then nothing changes.
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    let output = test_env.run_jj_in(&repo_path, ["squash", "-r", "b", "nonexistent"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Nothing changed.
    [EOF]
    ");

    // We get a warning if we pass a positional argument that looks like a revset
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    let output = test_env.run_jj_in(&repo_path, ["squash", "b"]);
    insta::assert_snapshot!(output, @r#"
    ------- stderr -------
    Warning: The argument "b" is being interpreted as a fileset expression. To specify a revset, pass -r "b" instead.
    Nothing changed.
    [EOF]
    "#);
}

#[test]
fn test_squash_keep_emptied() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "a"])
        .success();
    std::fs::write(repo_path.join("file1"), "a\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "b"])
        .success();
    std::fs::write(repo_path.join("file1"), "b\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "c"])
        .success();
    std::fs::write(repo_path.join("file1"), "c\n").unwrap();
    // Test the setup

    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  382c9bad7d42 c
    ○  d5d59175b481 b
    ○  184ddbcce5a9 a
    ◆  000000000000 (empty)
    [EOF]
    ");

    let output = test_env.run_jj_in(&repo_path, ["squash", "-r", "b", "--keep-emptied"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 descendant commits
    Working copy  (@) now at: mzvwutvl 7ee7f18a c | (no description set)
    Parent commit (@-)      : kkmpptxz 9490bd7f b | (empty) (no description set)
    [EOF]
    ");
    // With --keep-emptied, b remains even though it is now empty.
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  7ee7f18a5223 c
    ○  9490bd7f1e6a b (empty)
    ○  53bf93080518 a
    ◆  000000000000 (empty)
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1", "-r", "a"]);
    insta::assert_snapshot!(output, @r"
    b
    [EOF]
    ");
}

#[test]
fn test_squash_from_to() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    // Create history like this:
    // F
    // |
    // E C
    // | |
    // D B
    // |/
    // A
    //
    // When moving changes between e.g. C and F, we should not get unrelated changes
    // from B and D.
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "a"])
        .success();
    std::fs::write(repo_path.join("file1"), "a\n").unwrap();
    std::fs::write(repo_path.join("file2"), "a\n").unwrap();
    std::fs::write(repo_path.join("file3"), "a\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "b"])
        .success();
    std::fs::write(repo_path.join("file3"), "b\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "c"])
        .success();
    std::fs::write(repo_path.join("file1"), "c\n").unwrap();
    test_env.run_jj_in(&repo_path, ["edit", "a"]).success();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "d"])
        .success();
    std::fs::write(repo_path.join("file3"), "d\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "e"])
        .success();
    std::fs::write(repo_path.join("file2"), "e\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "f"])
        .success();
    std::fs::write(repo_path.join("file2"), "f\n").unwrap();
    // Test the setup
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  a847ab4967fe f
    ○  c2f9de87325d e
    ○  e0dac715116f d
    │ ○  59597b34a0d8 c
    │ ○  12d6103dc0c8 b
    ├─╯
    ○  b7b767179c44 a
    ◆  000000000000 (empty)
    [EOF]
    ");

    // Errors out if source and destination are the same
    let output = test_env.run_jj_in(&repo_path, ["squash", "--into", "@"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Source and destination cannot be the same
    [EOF]
    [exit status: 1]
    ");

    // Can squash from sibling, which results in the source being abandoned
    let output = test_env.run_jj_in(&repo_path, ["squash", "--from", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: kmkuslsw b902d1dd f | (no description set)
    Parent commit (@-)      : znkkpsqq c2f9de87 e | (no description set)
    Added 0 files, modified 1 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  b902d1dd59d9 f
    ○  c2f9de87325d e
    ○  e0dac715116f d
    │ ○  12d6103dc0c8 b c
    ├─╯
    ○  b7b767179c44 a
    ◆  000000000000 (empty)
    [EOF]
    ");
    // The change from the source has been applied
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1"]);
    insta::assert_snapshot!(output, @r"
    c
    [EOF]
    ");
    // File `file2`, which was not changed in source, is unchanged
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file2"]);
    insta::assert_snapshot!(output, @r"
    f
    [EOF]
    ");

    // Can squash from ancestor
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    let output = test_env.run_jj_in(&repo_path, ["squash", "--from", "@--"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: kmkuslsw cfc5eb87 f | (no description set)
    Parent commit (@-)      : znkkpsqq 4dc7c279 e | (no description set)
    [EOF]
    ");
    // The change has been removed from the source (the change pointed to by 'd'
    // became empty and was abandoned)
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  cfc5eb876eb1 f
    ○  4dc7c27994bd e
    │ ○  59597b34a0d8 c
    │ ○  12d6103dc0c8 b
    ├─╯
    ○  b7b767179c44 a d
    ◆  000000000000 (empty)
    [EOF]
    ");
    // The change from the source has been applied (the file contents were already
    // "f", as is typically the case when moving changes from an ancestor)
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file2"]);
    insta::assert_snapshot!(output, @r"
    f
    [EOF]
    ");

    // Can squash from descendant
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    let output = test_env.run_jj_in(&repo_path, ["squash", "--from", "e", "--into", "d"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 descendant commits
    Working copy  (@) now at: kmkuslsw 6de62c22 f | (no description set)
    Parent commit (@-)      : vruxwmqv 32196a11 d e | (no description set)
    [EOF]
    ");
    // The change has been removed from the source (the change pointed to by 'e'
    // became empty and was abandoned)
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  6de62c22fa07 f
    ○  32196a117ee3 d e
    │ ○  59597b34a0d8 c
    │ ○  12d6103dc0c8 b
    ├─╯
    ○  b7b767179c44 a
    ◆  000000000000 (empty)
    [EOF]
    ");
    // The change from the source has been applied
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file2", "-r", "d"]);
    insta::assert_snapshot!(output, @r"
    e
    [EOF]
    ");
}

#[test]
fn test_squash_from_to_partial() {
    let mut test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    // Create history like this:
    //   C
    //   |
    // D B
    // |/
    // A
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "a"])
        .success();
    std::fs::write(repo_path.join("file1"), "a\n").unwrap();
    std::fs::write(repo_path.join("file2"), "a\n").unwrap();
    std::fs::write(repo_path.join("file3"), "a\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "b"])
        .success();
    std::fs::write(repo_path.join("file3"), "b\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "c"])
        .success();
    std::fs::write(repo_path.join("file1"), "c\n").unwrap();
    std::fs::write(repo_path.join("file2"), "c\n").unwrap();
    test_env.run_jj_in(&repo_path, ["edit", "a"]).success();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "d"])
        .success();
    std::fs::write(repo_path.join("file3"), "d\n").unwrap();
    // Test the setup
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  e0dac715116f d
    │ ○  087591be5a01 c
    │ ○  12d6103dc0c8 b
    ├─╯
    ○  b7b767179c44 a
    ◆  000000000000 (empty)
    [EOF]
    ");

    let edit_script = test_env.set_up_fake_diff_editor();

    // If we don't make any changes in the diff-editor, the whole change is moved
    let output = test_env.run_jj_in(&repo_path, ["squash", "-i", "--from", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: vruxwmqv 987bcfb2 d | (no description set)
    Parent commit (@-)      : qpvuntsm b7b76717 a | (no description set)
    Added 0 files, modified 2 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  987bcfb2eb62 d
    │ ○  12d6103dc0c8 b c
    ├─╯
    ○  b7b767179c44 a
    ◆  000000000000 (empty)
    [EOF]
    ");
    // The changes from the source has been applied
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1"]);
    insta::assert_snapshot!(output, @r"
    c
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file2"]);
    insta::assert_snapshot!(output, @r"
    c
    [EOF]
    ");
    // File `file3`, which was not changed in source, is unchanged
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file3"]);
    insta::assert_snapshot!(output, @r"
    d
    [EOF]
    ");

    // Can squash only part of the change in interactive mode
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    std::fs::write(&edit_script, "reset file2").unwrap();
    let output = test_env.run_jj_in(&repo_path, ["squash", "-i", "--from", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: vruxwmqv 576244e8 d | (no description set)
    Parent commit (@-)      : qpvuntsm b7b76717 a | (no description set)
    Added 0 files, modified 1 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  576244e87883 d
    │ ○  6f486f2f4539 c
    │ ○  12d6103dc0c8 b
    ├─╯
    ○  b7b767179c44 a
    ◆  000000000000 (empty)
    [EOF]
    ");
    // The selected change from the source has been applied
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1"]);
    insta::assert_snapshot!(output, @r"
    c
    [EOF]
    ");
    // The unselected change from the source has not been applied
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file2"]);
    insta::assert_snapshot!(output, @r"
    a
    [EOF]
    ");
    // File `file3`, which was changed in source's parent, is unchanged
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file3"]);
    insta::assert_snapshot!(output, @r"
    d
    [EOF]
    ");

    // Can squash only part of the change from a sibling in non-interactive mode
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    // Clear the script so we know it won't be used
    std::fs::write(&edit_script, "").unwrap();
    let output = test_env.run_jj_in(&repo_path, ["squash", "--from", "c", "file1"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: vruxwmqv 5b407c24 d | (no description set)
    Parent commit (@-)      : qpvuntsm b7b76717 a | (no description set)
    Added 0 files, modified 1 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  5b407c249fa7 d
    │ ○  724d64da1487 c
    │ ○  12d6103dc0c8 b
    ├─╯
    ○  b7b767179c44 a
    ◆  000000000000 (empty)
    [EOF]
    ");
    // The selected change from the source has been applied
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1"]);
    insta::assert_snapshot!(output, @r"
    c
    [EOF]
    ");
    // The unselected change from the source has not been applied
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file2"]);
    insta::assert_snapshot!(output, @r"
    a
    [EOF]
    ");
    // File `file3`, which was changed in source's parent, is unchanged
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file3"]);
    insta::assert_snapshot!(output, @r"
    d
    [EOF]
    ");

    // Can squash only part of the change from a descendant in non-interactive mode
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    // Clear the script so we know it won't be used
    std::fs::write(&edit_script, "").unwrap();
    let output = test_env.run_jj_in(
        &repo_path,
        ["squash", "--from", "c", "--into", "b", "file1"],
    );
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 descendant commits
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  e0dac715116f d
    │ ○  d2a587ae205d c
    │ ○  a53394306362 b
    ├─╯
    ○  b7b767179c44 a
    ◆  000000000000 (empty)
    [EOF]
    ");
    // The selected change from the source has been applied
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file1", "-r", "b"]);
    insta::assert_snapshot!(output, @r"
    c
    [EOF]
    ");
    // The unselected change from the source has not been applied
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "file2", "-r", "b"]);
    insta::assert_snapshot!(output, @r"
    a
    [EOF]
    ");

    // If we specify only a non-existent file, then nothing changes.
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    let output = test_env.run_jj_in(&repo_path, ["squash", "--from", "c", "nonexistent"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Nothing changed.
    [EOF]
    ");
}

#[test]
fn test_squash_from_multiple() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    // Create history like this:
    //   F
    //   |
    //   E
    //  /|\
    // B C D
    //  \|/
    //   A
    let file = repo_path.join("file");
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "a"])
        .success();
    std::fs::write(&file, "a\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "b"])
        .success();
    std::fs::write(&file, "b\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new", "@-"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "c"])
        .success();
    std::fs::write(&file, "c\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new", "@-"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "d"])
        .success();
    std::fs::write(&file, "d\n").unwrap();
    test_env
        .run_jj_in(&repo_path, ["new", "all:visible_heads()"])
        .success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "e"])
        .success();
    std::fs::write(&file, "e\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "f"])
        .success();
    std::fs::write(&file, "f\n").unwrap();
    // Test the setup
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  94e57ecb8d4f f
    ○      78ed28eb87b8 e
    ├─┬─╮
    │ │ ○  35e764e4357c b
    │ ○ │  02a128cd4344 c
    │ ├─╯
    ○ │  aaf7b53a1b64 d
    ├─╯
    ○  3b1673b6370c a
    ◆  000000000000 (empty)
    [EOF]
    ");

    // Squash a few commits sideways
    let output = test_env.run_jj_in(&repo_path, ["squash", "--from=b", "--from=c", "--into=d"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 descendant commits
    Working copy  (@) now at: kpqxywon 7ea39167 f | (no description set)
    Parent commit (@-)      : yostqsxw acfbf2a0 e | (no description set)
    New conflicts appeared in 1 commits:
      yqosqzyt 4df3b215 d | (conflict) (no description set)
    Hint: To resolve the conflicts, start by updating to it:
      jj new yqosqzyt
    Then use `jj resolve`, or edit the conflict markers in the file directly.
    Once the conflicts are resolved, you may want to inspect the result with `jj diff`.
    Then run `jj squash` to move the resolution into the conflicted commit.
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  7ea391676d52 f
    ○    acfbf2a0600d e
    ├─╮
    × │  4df3b2156c3d d
    ├─╯
    ○  3b1673b6370c a b c
    ◆  000000000000 (empty)
    [EOF]
    ");
    // The changes from the sources have been applied
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "-r=d", "file"]);
    insta::assert_snapshot!(output, @r"
    <<<<<<< Conflict 1 of 1
    %%%%%%% Changes from base #1 to side #1
    -a
    +d
    %%%%%%% Changes from base #2 to side #2
    -a
    +b
    +++++++ Contents of side #3
    c
    >>>>>>> Conflict 1 of 1 ends
    [EOF]
    ");

    // Squash a few commits up an down
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    let output = test_env.run_jj_in(&repo_path, ["squash", "--from=b|c|f", "--into=e"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 descendant commits
    Working copy  (@) now at: xznxytkn 6a670d1a (empty) (no description set)
    Parent commit (@-)      : yostqsxw c1293ff7 e f | (no description set)
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  6a670d1ac76e (empty)
    ○    c1293ff7be51 e f
    ├─╮
    ○ │  aaf7b53a1b64 d
    ├─╯
    ○  3b1673b6370c a b c
    ◆  000000000000 (empty)
    [EOF]
    ");
    // The changes from the sources have been applied to the destination
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "-r=e", "file"]);
    insta::assert_snapshot!(output, @r"
    f
    [EOF]
    ");

    // Empty squash shouldn't crash
    let output = test_env.run_jj_in(&repo_path, ["squash", "--from=none()"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Nothing changed.
    [EOF]
    ");
}

#[test]
fn test_squash_from_multiple_partial() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    // Create history like this:
    //   F
    //   |
    //   E
    //  /|\
    // B C D
    //  \|/
    //   A
    let file1 = repo_path.join("file1");
    let file2 = repo_path.join("file2");
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "a"])
        .success();
    std::fs::write(&file1, "a\n").unwrap();
    std::fs::write(&file2, "a\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "b"])
        .success();
    std::fs::write(&file1, "b\n").unwrap();
    std::fs::write(&file2, "b\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new", "@-"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "c"])
        .success();
    std::fs::write(&file1, "c\n").unwrap();
    std::fs::write(&file2, "c\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new", "@-"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "d"])
        .success();
    std::fs::write(&file1, "d\n").unwrap();
    std::fs::write(&file2, "d\n").unwrap();
    test_env
        .run_jj_in(&repo_path, ["new", "all:visible_heads()"])
        .success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "e"])
        .success();
    std::fs::write(&file1, "e\n").unwrap();
    std::fs::write(&file2, "e\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    test_env
        .run_jj_in(&repo_path, ["bookmark", "create", "-r@", "f"])
        .success();
    std::fs::write(&file1, "f\n").unwrap();
    std::fs::write(&file2, "f\n").unwrap();
    // Test the setup
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  30980b9045f7 f
    ○      5326a04aac1f e
    ├─┬─╮
    │ │ ○  d117da276a0f b
    │ ○ │  93a7bfff61e7 c
    │ ├─╯
    ○ │  763809ca0131 d
    ├─╯
    ○  54d3c1c0e9fd a
    ◆  000000000000 (empty)
    [EOF]
    ");

    // Partially squash a few commits sideways
    let output = test_env.run_jj_in(&repo_path, ["squash", "--from=b|c", "--into=d", "file1"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 descendant commits
    Working copy  (@) now at: kpqxywon a8530305 f | (no description set)
    Parent commit (@-)      : yostqsxw 0a3637fc e | (no description set)
    New conflicts appeared in 1 commits:
      yqosqzyt 05a3ab3d d | (conflict) (no description set)
    Hint: To resolve the conflicts, start by updating to it:
      jj new yqosqzyt
    Then use `jj resolve`, or edit the conflict markers in the file directly.
    Once the conflicts are resolved, you may want to inspect the result with `jj diff`.
    Then run `jj squash` to move the resolution into the conflicted commit.
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  a8530305127c f
    ○      0a3637fca632 e
    ├─┬─╮
    │ │ ○  450d1499c1ae b
    │ ○ │  14b44bf0473c c
    │ ├─╯
    × │  05a3ab3dffc8 d
    ├─╯
    ○  54d3c1c0e9fd a
    ◆  000000000000 (empty)
    [EOF]
    ");
    // The selected changes have been removed from the sources
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "-r=b", "file1"]);
    insta::assert_snapshot!(output, @r"
    a
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "-r=c", "file1"]);
    insta::assert_snapshot!(output, @r"
    a
    [EOF]
    ");
    // The selected changes from the sources have been applied
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "-r=d", "file1"]);
    insta::assert_snapshot!(output, @r"
    <<<<<<< Conflict 1 of 1
    %%%%%%% Changes from base #1 to side #1
    -a
    +d
    %%%%%%% Changes from base #2 to side #2
    -a
    +b
    +++++++ Contents of side #3
    c
    >>>>>>> Conflict 1 of 1 ends
    [EOF]
    ");
    // The unselected change from the sources have not been applied to the
    // destination
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "-r=d", "file2"]);
    insta::assert_snapshot!(output, @r"
    d
    [EOF]
    ");

    // Partially squash a few commits up an down
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    let output = test_env.run_jj_in(&repo_path, ["squash", "--from=b|c|f", "--into=e", "file1"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 descendant commits
    Working copy  (@) now at: kpqxywon 3b7559b8 f | (no description set)
    Parent commit (@-)      : yostqsxw a3b1714c e | (no description set)
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  3b7559b89a57 f
    ○      a3b1714cdfb2 e
    ├─┬─╮
    │ │ ○  867efb38e801 b
    │ ○ │  84dcb3d4b3eb c
    │ ├─╯
    ○ │  763809ca0131 d
    ├─╯
    ○  54d3c1c0e9fd a
    ◆  000000000000 (empty)
    [EOF]
    ");
    // The selected changes have been removed from the sources
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "-r=b", "file1"]);
    insta::assert_snapshot!(output, @r"
    a
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "-r=c", "file1"]);
    insta::assert_snapshot!(output, @r"
    a
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "-r=f", "file1"]);
    insta::assert_snapshot!(output, @r"
    f
    [EOF]
    ");
    // The selected changes from the sources have been applied to the destination
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "-r=e", "file1"]);
    insta::assert_snapshot!(output, @r"
    f
    [EOF]
    ");
    // The unselected changes from the sources have not been applied
    let output = test_env.run_jj_in(&repo_path, ["file", "show", "-r=d", "file2"]);
    insta::assert_snapshot!(output, @r"
    d
    [EOF]
    ");
}

#[test]
fn test_squash_from_multiple_partial_no_op() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    // Create history like this:
    // B C D
    //  \|/
    //   A
    let file_a = repo_path.join("a");
    let file_b = repo_path.join("b");
    let file_c = repo_path.join("c");
    let file_d = repo_path.join("d");
    test_env
        .run_jj_in(&repo_path, ["describe", "-m=a"])
        .success();
    std::fs::write(file_a, "a\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new", "-m=b"]).success();
    std::fs::write(file_b, "b\n").unwrap();
    test_env
        .run_jj_in(&repo_path, ["new", "@-", "-m=c"])
        .success();
    std::fs::write(file_c, "c\n").unwrap();
    test_env
        .run_jj_in(&repo_path, ["new", "@-", "-m=d"])
        .success();
    std::fs::write(file_d, "d\n").unwrap();
    // Test the setup
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  b37ca1ee3306 d
    │ ○  f40b442af3e8 c
    ├─╯
    │ ○  b73077b08c59 b
    ├─╯
    ○  2443ea76b0b1 a
    ◆  000000000000 (empty)
    [EOF]
    ");

    // Source commits that didn't match the paths are not rewritten
    let output = test_env.run_jj_in(
        &repo_path,
        ["squash", "--from=@-+ ~ @", "--into=@", "-m=d", "b"],
    );
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: mzvwutvl e178068a d
    Parent commit (@-)      : qpvuntsm 2443ea76 a
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  e178068add8c d
    │ ○  f40b442af3e8 c
    ├─╯
    ○  2443ea76b0b1 a
    ◆  000000000000 (empty)
    [EOF]
    ");
    let output = test_env.run_jj_in(
        &repo_path,
        [
            "evolog",
            "-T",
            r#"separate(" ", commit_id.short(), description)"#,
        ],
    );
    insta::assert_snapshot!(output, @r"
    @    e178068add8c d
    ├─╮
    │ ○  b73077b08c59 b
    │ ○  a786561e909f b
    ○  b37ca1ee3306 d
    ○  1d9eb34614c9 d
    [EOF]
    ");

    // If no source commits match the paths, then the whole operation is a no-op
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    let output = test_env.run_jj_in(
        &repo_path,
        ["squash", "--from=@-+ ~ @", "--into=@", "-m=d", "a"],
    );
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  b37ca1ee3306 d
    │ ○  f40b442af3e8 c
    ├─╯
    │ ○  b73077b08c59 b
    ├─╯
    ○  2443ea76b0b1 a
    ◆  000000000000 (empty)
    [EOF]
    ");
}

#[must_use]
fn get_log_output(test_env: &TestEnvironment, repo_path: &Path) -> CommandOutput {
    let template = r#"separate(
        " ",
        commit_id.short(),
        bookmarks,
        description,
        if(empty, "(empty)")
    )"#;
    test_env.run_jj_in(repo_path, ["log", "-T", template])
}

#[test]
fn test_squash_description() {
    let mut test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    let edit_script = test_env.set_up_fake_editor();
    std::fs::write(&edit_script, r#"fail"#).unwrap();

    // If both descriptions are empty, the resulting description is empty
    std::fs::write(repo_path.join("file1"), "a\n").unwrap();
    std::fs::write(repo_path.join("file2"), "a\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    std::fs::write(repo_path.join("file1"), "b\n").unwrap();
    std::fs::write(repo_path.join("file2"), "b\n").unwrap();
    test_env.run_jj_in(&repo_path, ["squash"]).success();
    insta::assert_snapshot!(get_description(&test_env, &repo_path, "@-"), @"");

    // If the destination's description is empty and the source's description is
    // non-empty, the resulting description is from the source
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    test_env
        .run_jj_in(&repo_path, ["describe", "-m", "source"])
        .success();
    test_env.run_jj_in(&repo_path, ["squash"]).success();
    insta::assert_snapshot!(get_description(&test_env, &repo_path, "@-"), @r"
    source
    [EOF]
    ");

    // If the destination description is non-empty and the source's description is
    // empty, the resulting description is from the destination
    test_env
        .run_jj_in(&repo_path, ["op", "restore", "@--"])
        .success();
    test_env
        .run_jj_in(&repo_path, ["describe", "@-", "-m", "destination"])
        .success();
    test_env.run_jj_in(&repo_path, ["squash"]).success();
    insta::assert_snapshot!(get_description(&test_env, &repo_path, "@-"), @r"
    destination
    [EOF]
    ");

    // An explicit description on the command-line overrides this
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    test_env
        .run_jj_in(&repo_path, ["squash", "-m", "custom"])
        .success();
    insta::assert_snapshot!(get_description(&test_env, &repo_path, "@-"), @r"
    custom
    [EOF]
    ");

    // If both descriptions were non-empty, we get asked for a combined description
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    test_env
        .run_jj_in(&repo_path, ["describe", "-m", "source"])
        .success();
    std::fs::write(&edit_script, "dump editor0").unwrap();
    test_env.run_jj_in(&repo_path, ["squash"]).success();
    insta::assert_snapshot!(get_description(&test_env, &repo_path, "@-"), @r"
    destination

    source
    [EOF]
    ");
    insta::assert_snapshot!(
        std::fs::read_to_string(test_env.env_root().join("editor0")).unwrap(), @r#"
    JJ: Enter a description for the combined commit.
    JJ: Description from the destination commit:
    destination

    JJ: Description from source commit:
    source

    JJ: This commit contains the following changes:
    JJ:     A file1
    JJ:     A file2
    JJ:
    JJ: Lines starting with "JJ:" (like this one) will be removed.
    "#);

    // An explicit description on the command-line overrides prevents launching an
    // editor
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    test_env
        .run_jj_in(&repo_path, ["squash", "-m", "custom"])
        .success();
    insta::assert_snapshot!(get_description(&test_env, &repo_path, "@-"), @r"
    custom
    [EOF]
    ");

    // If the source's *content* doesn't become empty, then the source remains and
    // both descriptions are unchanged
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    test_env
        .run_jj_in(&repo_path, ["squash", "file1"])
        .success();
    insta::assert_snapshot!(get_description(&test_env, &repo_path, "@-"), @r"
    destination
    [EOF]
    ");
    insta::assert_snapshot!(get_description(&test_env, &repo_path, "@"), @r"
    source
    [EOF]
    ");
}

#[test]
fn test_squash_description_editor_avoids_unc() {
    let mut test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    let edit_script = test_env.set_up_fake_editor();
    std::fs::write(repo_path.join("file1"), "a\n").unwrap();
    std::fs::write(repo_path.join("file2"), "a\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    std::fs::write(repo_path.join("file1"), "b\n").unwrap();
    std::fs::write(repo_path.join("file2"), "b\n").unwrap();
    test_env
        .run_jj_in(&repo_path, ["describe", "@-", "-m", "destination"])
        .success();
    test_env
        .run_jj_in(&repo_path, ["describe", "-m", "source"])
        .success();

    std::fs::write(edit_script, "dump-path path").unwrap();
    test_env.run_jj_in(&repo_path, ["squash"]).success();

    let edited_path =
        PathBuf::from(std::fs::read_to_string(test_env.env_root().join("path")).unwrap());
    // While `assert!(!edited_path.starts_with("//?/"))` could work here in most
    // cases, it fails when it is not safe to strip the prefix, such as paths
    // over 260 chars.
    assert_eq!(edited_path, dunce::simplified(&edited_path));
}

#[test]
fn test_squash_empty() {
    let mut test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    test_env
        .run_jj_in(&repo_path, ["commit", "-m", "parent"])
        .success();

    let output = test_env.run_jj_in(&repo_path, ["squash"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Working copy  (@) now at: kkmpptxz adece6e8 (empty) (no description set)
    Parent commit (@-)      : qpvuntsm 5076fc41 (empty) parent
    [EOF]
    ");
    insta::assert_snapshot!(get_description(&test_env, &repo_path, "@-"), @r"
    parent
    [EOF]
    ");

    test_env
        .run_jj_in(&repo_path, ["describe", "-m", "child"])
        .success();
    test_env.set_up_fake_editor();
    test_env.run_jj_in(&repo_path, ["squash"]).success();
    insta::assert_snapshot!(get_description(&test_env, &repo_path, "@-"), @r"
    parent

    child
    [EOF]
    ");
}

#[test]
fn test_squash_use_destination_message() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    test_env.run_jj_in(&repo_path, ["commit", "-m=a"]).success();
    test_env.run_jj_in(&repo_path, ["commit", "-m=b"]).success();
    test_env
        .run_jj_in(&repo_path, ["describe", "-m=c"])
        .success();
    // Test the setup
    insta::assert_snapshot!(get_log_output_with_description(&test_env, &repo_path), @r"
    @  8aac283daeac c
    ○  017c7f689ed7 b
    ○  d8d5f980a897 a
    ◆  000000000000
    [EOF]
    ");

    // Squash the current revision using the short name for the option.
    test_env.run_jj_in(&repo_path, ["squash", "-u"]).success();
    insta::assert_snapshot!(get_log_output_with_description(&test_env, &repo_path), @r"
    @  fd33e4bc332b
    ○  3a17aa5dcce9 b
    ○  d8d5f980a897 a
    ◆  000000000000
    [EOF]
    ");

    // Undo and squash again, but this time squash both "b" and "c" into "a".
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    test_env
        .run_jj_in(
            &repo_path,
            [
                "squash",
                "--use-destination-message",
                "--from",
                "description(b)::",
                "--into",
                "description(a)",
            ],
        )
        .success();
    insta::assert_snapshot!(get_log_output_with_description(&test_env, &repo_path), @r"
    @  7c832accbf60
    ○  688660377651 a
    ◆  000000000000
    [EOF]
    ");
}

// The --use-destination-message and --message options are incompatible.
#[test]
fn test_squash_use_destination_message_and_message_mutual_exclusion() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");
    test_env.run_jj_in(&repo_path, ["commit", "-m=a"]).success();
    test_env
        .run_jj_in(&repo_path, ["describe", "-m=b"])
        .success();
    insta::assert_snapshot!(test_env.run_jj_in(
        &repo_path,
        [
            "squash",
            "--message=123",
            "--use-destination-message",
        ],
    ), @r"
    ------- stderr -------
    error: the argument '--message <MESSAGE>' cannot be used with '--use-destination-message'

    Usage: jj squash --message <MESSAGE> [FILESETS]...

    For more information, try '--help'.
    [EOF]
    [exit status: 2]
    ");
}

#[must_use]
fn get_description(test_env: &TestEnvironment, repo_path: &Path, rev: &str) -> CommandOutput {
    test_env.run_jj_in(
        repo_path,
        ["log", "--no-graph", "-T", "description", "-r", rev],
    )
}

#[must_use]
fn get_log_output_with_description(test_env: &TestEnvironment, repo_path: &Path) -> CommandOutput {
    let template = r#"separate(" ", commit_id.short(), description)"#;
    test_env.run_jj_in(repo_path, ["log", "-T", template])
}
