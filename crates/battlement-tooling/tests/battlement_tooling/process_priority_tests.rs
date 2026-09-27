#![cfg(unix)]

use std::process::Command;

use battlement_tooling::process_priority;

#[test]
fn build_children_inherit_reduced_priority_without_changing_the_controller() {
  let probe = "ps -o nice= -p $$; sh -c 'ps -o nice= -p $$'";
  let before = Command::new("sh").args(["-c", probe]).output().unwrap();
  let output = process_priority::command("sh")
    .args(["-c", probe])
    .output()
    .unwrap();
  assert!(output.status.success());
  let priorities: Vec<i32> = String::from_utf8(output.stdout)
    .unwrap()
    .split_whitespace()
    .map(|value| value.parse().unwrap())
    .collect();
  assert_eq!(priorities.len(), 2);
  assert!(priorities.iter().all(|value| *value >= 10));
  let after = Command::new("sh").args(["-c", probe]).output().unwrap();
  assert_eq!(before.stdout, after.stdout);
}
