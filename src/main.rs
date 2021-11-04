use anyhow::bail;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs, thread};

fn main() -> anyhow::Result<()> {
	for arg in env::args() {
		match arg.as_str() {
			"--help" => {
				print_help();
				return Ok(());
			}
			_ => (),
		}
	}

	let root = env::args()
		.last()
		.map(PathBuf::from)
		.filter(|path| path.is_dir());

	cargo_clean_subdirs(root)
}

fn print_help() {
	const USAGE: &str = r#"Usage:
"#;
	println!("{}", USAGE);
}

fn cargo_clean_subdirs(root: Option<PathBuf>) -> anyhow::Result<()> {
	let root = root.unwrap_or_else(|| PathBuf::from("."));
	let dir_iter = fs::read_dir(root)?
		.filter_map(Result::ok)
		.filter(|entry| entry.path().is_dir());

	const THREAD_COUNT: usize = 8;
	let mut thread_handles = Vec::with_capacity(THREAD_COUNT);
	let (mut sender, receiver) = spmc::channel();

	for _ in 0..THREAD_COUNT {
		let receiver = receiver.clone();
		let handle = thread::spawn(move || {
			while let Ok(dir_path) = receiver.recv() {
				if let Err(e) = run_cargo_clean(dir_path) {
					eprintln!("{:?}", e);
				}
			}
		});
		thread_handles.push(handle);
	}

	dir_iter.for_each(|dir| {
		let _ = sender.send(dir.path());
	});
	drop(sender);
	for handle in thread_handles {
		let _ = handle.join();
	}

	Ok(())
}

fn run_cargo_clean(dir: PathBuf) -> anyhow::Result<()> {
	println!("Running `cargo clean` in {}", dir.display());
	let output = Command::new("cargo")
		.arg("clean")
		.current_dir(dir)
		.output()?;

	if !output.status.success() {
		bail!("{}", String::from_utf8_lossy(&output.stderr));
	}

	Ok(())
}
