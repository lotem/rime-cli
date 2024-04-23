use crate::package::配方包;

use std::path::{Path, PathBuf};

pub fn 下載配方包(包: &配方包) -> anyhow::Result<()> {
    log::debug!("下載配方包: {}, 位於 {}", 包.配方, 包.倉庫);
    let 本地倉庫路徑: PathBuf = ["pkg", 包.配方.方家.as_str(), 包.配方.名字.as_str()]
        .iter()
        .collect();
    if 本地倉庫路徑.exists() {
        同步既存倉庫(包, &本地倉庫路徑)
    } else {
        搬運倉庫(包, &本地倉庫路徑)
    }
}

fn 搬運倉庫(包: &配方包, 本地路徑: &Path) -> anyhow::Result<()> {
    let 網址 = &包.倉庫.網址;
    let _倉庫 = git::clone(網址, 本地路徑)?;
    Ok(())
}

fn 同步既存倉庫(包: &配方包, 本地路徑: &Path) -> anyhow::Result<()> {
    const 遠端代號: &str = "origin";
    let 遠端分支 = 包
        .倉庫
        .分支
        .as_ref()
        .map(String::as_str)
        .unwrap_or("master");
    git::pull(本地路徑, 遠端代號, 遠端分支)?;
    Ok(())
}

mod git {
    use git2::build::{CheckoutBuilder, RepoBuilder};
    use git2::{
        AnnotatedCommit, AutotagOption, ErrorClass, ErrorCode, FetchOptions, Progress, Reference,
        Remote, RemoteCallbacks, Repository,
    };
    use std::cell::RefCell;
    use std::io::{self, Write};
    use std::path::{Path, PathBuf};

    struct State {
        progress: Option<Progress<'static>>,
        total: usize,
        current: usize,
        path: Option<PathBuf>,
        newline: bool,
    }

    fn print(state: &mut State) {
        let stats = state.progress.as_ref().unwrap();
        let network_pct = (100 * stats.received_objects()) / stats.total_objects();
        let index_pct = (100 * stats.indexed_objects()) / stats.total_objects();
        let co_pct = if state.total > 0 {
            (100 * state.current) / state.total
        } else {
            0
        };
        let kbytes = stats.received_bytes() / 1024;
        if stats.received_objects() == stats.total_objects() {
            if !state.newline {
                println!();
                state.newline = true;
            }
            print!(
                "Resolving deltas {}/{}\r",
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else {
            print!(
                "net {:3}% ({:4} kb, {:5}/{:5})  /  idx {:3}% ({:5}/{:5})  \
             /  chk {:3}% ({:4}/{:4}) {}\r",
                network_pct,
                kbytes,
                stats.received_objects(),
                stats.total_objects(),
                index_pct,
                stats.indexed_objects(),
                stats.total_objects(),
                co_pct,
                state.current,
                state.total,
                state
                    .path
                    .as_ref()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            )
        }
        io::stdout().flush().unwrap();
    }

    pub fn clone(url: &str, path: &Path) -> Result<(), git2::Error> {
        let state = RefCell::new(State {
            progress: None,
            total: 0,
            current: 0,
            path: None,
            newline: false,
        });
        let mut cb = RemoteCallbacks::new();
        cb.transfer_progress(|stats| {
            let mut state = state.borrow_mut();
            state.progress = Some(stats.to_owned());
            print(&mut *state);
            true
        });

        let mut co = CheckoutBuilder::new();
        co.progress(|path, cur, total| {
            let mut state = state.borrow_mut();
            state.path = path.map(|p| p.to_path_buf());
            state.current = cur;
            state.total = total;
            print(&mut *state);
        });

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(cb);
        RepoBuilder::new()
            .fetch_options(fo)
            .with_checkout(co)
            .clone(url, path)?;
        println!();

        Ok(())
    }

    fn do_fetch<'a>(
        repo: &'a Repository,
        refs: &[&str],
        remote: &'a mut Remote,
    ) -> Result<AnnotatedCommit<'a>, git2::Error> {
        let mut cb = RemoteCallbacks::new();

        // Print out our transfer progress.
        cb.transfer_progress(|stats| {
            if stats.received_objects() == stats.total_objects() {
                print!(
                    "Resolving deltas {}/{}\r",
                    stats.indexed_deltas(),
                    stats.total_deltas()
                );
            } else if stats.total_objects() > 0 {
                print!(
                    "Received {}/{} objects ({}) in {} bytes\r",
                    stats.received_objects(),
                    stats.total_objects(),
                    stats.indexed_objects(),
                    stats.received_bytes()
                );
            }
            io::stdout().flush().unwrap();
            true
        });

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(cb);
        // Always fetch all tags.
        // Perform a download and also update tips
        fo.download_tags(AutotagOption::All);
        println!("Fetching {} for repo", remote.name().unwrap());
        remote.fetch(refs, Some(&mut fo), None)?;

        // If there are local objects (we got a thin pack), then tell the user
        // how many objects we saved from having to cross the network.
        let stats = remote.stats();
        if stats.local_objects() > 0 {
            println!(
                "\rReceived {}/{} objects in {} bytes (used {} local \
                objects)",
                stats.indexed_objects(),
                stats.total_objects(),
                stats.received_bytes(),
                stats.local_objects()
            );
        } else {
            println!(
                "\rReceived {}/{} objects in {} bytes",
                stats.indexed_objects(),
                stats.total_objects(),
                stats.received_bytes()
            );
        }

        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        Ok(repo.reference_to_annotated_commit(&fetch_head)?)
    }

    fn fast_forward(
        repo: &Repository,
        lb: &mut Reference,
        rc: &AnnotatedCommit,
    ) -> Result<(), git2::Error> {
        let name = match lb.name() {
            Some(s) => s.to_string(),
            None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
        };
        let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
        println!("{}", msg);
        lb.set_target(rc.id(), &msg)?;
        repo.set_head(&name)?;
        repo.checkout_head(Some(
            CheckoutBuilder::default()
                // For some reason the force is required to make the working directory actually get updated
                // I suspect we should be adding some logic to handle dirty working directory states
                // but this is just an example so maybe not.
                .force(),
        ))?;
        Ok(())
    }

    fn do_merge<'a>(
        repo: &'a Repository,
        remote_branch: &str,
        fetch_commit: AnnotatedCommit<'a>,
    ) -> Result<(), git2::Error> {
        // 1. do a merge analysis
        let analysis = repo.merge_analysis(&[&fetch_commit])?;

        // 2. Do the appropriate merge
        if analysis.0.is_fast_forward() {
            println!("Doing a fast forward");
            // do a fast forward
            let refname = format!("refs/heads/{}", remote_branch);
            match repo.find_reference(&refname) {
                Ok(mut r) => {
                    fast_forward(repo, &mut r, &fetch_commit)?;
                }
                Err(_) => {
                    // The branch doesn't exist so just set the reference to the
                    // commit directly. Usually this is because you are pulling
                    // into an empty repository.
                    repo.reference(
                        &refname,
                        fetch_commit.id(),
                        true,
                        &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                    )?;
                    repo.set_head(&refname)?;
                    repo.checkout_head(Some(
                        CheckoutBuilder::default()
                            .allow_conflicts(true)
                            .conflict_style_merge(true)
                            .force(),
                    ))?;
                }
            };
        } else if analysis.0.is_normal() {
            // will not do a normal merge
            let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
            return Err(git2::Error::new(
                ErrorCode::NotFastForward,
                ErrorClass::Repository,
                format!(
                    "Cannot fast-forward: HEAD {} -x-> FETCH_HEAD {}",
                    head_commit.id(),
                    fetch_commit.id()
                ),
            ));
        } else {
            println!("Nothing to do...");
        }
        Ok(())
    }

    pub fn pull(
        repo_path: &Path,
        remote_name: &str,
        remote_branch: &str,
    ) -> Result<(), git2::Error> {
        let repo = Repository::open(repo_path)?;
        let mut remote = repo.find_remote(remote_name)?;
        let fetch_commit = do_fetch(&repo, &[remote_branch], &mut remote)?;
        do_merge(&repo, &remote_branch, fetch_commit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::代碼庫地址;
    use crate::recipe::配方名片;

    #[ignore]
    #[test]
    fn 測試搬運倉庫() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let 本地測試路徑 = tmp_dir.path().join("test-clone");
        搬運倉庫(
            &配方包 {
                配方: 配方名片 {
                    方家: "test".to_string(),
                    名字: "test-clone".to_string(),
                    版本: None,
                },
                倉庫: 代碼庫地址 {
                    網址: "https://github.com/rime/rime-prelude.git".to_string(),
                    分支: None,
                },
            },
            &本地測試路徑,
        )?;
        Ok(())
    }
}
