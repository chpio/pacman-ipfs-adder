use std::process::{Command, Output, Stdio};
use std::fs::OpenOptions;
use std::io::prelude::*;

fn assert_cmd_output(name: &str, o: &Output) {
    if !o.status.success() {
        panic!("`{}` exited with status: {}\n{}",
               name,
               o.status,
               String::from_utf8_lossy(&o.stderr));
    }
}

fn main() {
    println!(">>> rsyncing...");
    {
        let rsync_out = Command::new("rsync")
            .args(&["-rtlH",
                    "--delete-after",
                    "--delay-updates",
                    "--copy-links",
                    "--safe-links",
                    "rsync://mirror.23media.de/archlinux/",
                    "/data/arch"])
            .stdout(Stdio::inherit())
            .output()
            .unwrap();
        assert_cmd_output("rsync", &rsync_out);
    }

    println!(">>> adding to ipfs...");
    let ipfs_hash = {
        let ipfs_out = Command::new("ipfs")
            .args(&["add", "--quiet", "--recursive", "/data/arch"])
            .output()
            .unwrap();
        assert_cmd_output("ipfs add", &ipfs_out);
        let ipfs_stdout = String::from_utf8_lossy(&ipfs_out.stdout);
        ipfs_stdout.lines().last().expect("No stdout from ipfs add").to_string()
    };
    println!(">>> ipfs hash: {:?}", ipfs_hash);

    {
        let hashes_file = OpenOptions::new()
            .read(true)
            .open("/data/pacman-ipfs-adder-hashes");
        if let Ok(mut hashes_file) = hashes_file {
            let mut content = String::new();
            hashes_file.read_to_string(&mut content).unwrap();
            for h in content.lines().filter(|h| h != &ipfs_hash) {
                println!(">>> unpinning: {:?}", h);
                Command::new("ipfs")
                    .args(&["pin", "rm", h])
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();
            }
        }
    }

    {
        let mut hashes_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open("/data/pacman-ipfs-adder-hashes")
            .unwrap();
        writeln!(hashes_file, "{}", ipfs_hash).unwrap();
    }

    println!(">>> publishing to ipns...");
    {
        let ipfs_out = Command::new("ipfs")
            .args(&["name", "publish", &ipfs_hash])
            .stdout(Stdio::inherit())
            .output()
            .unwrap();
        assert_cmd_output("ipfs publish", &ipfs_out);
    }
    println!(">>> done");
}
