use std::process::{Command, Output, Stdio};

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
                    "--info=progress2",
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
