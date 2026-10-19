use crate::package::配方包;
use crate::recipe::配方名片;

use anyhow::anyhow;
use std::path::Path;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct 下載參數 {
    /// 倉庫域名
    #[structopt(short, long)]
    host: Option<String>,
    /// 代理服務器地址
    #[structopt(short, long)]
    proxy: Option<String>,
    /// token 令牌
    #[structopt(short, long)]
    token: Option<String>,
}

impl 下載參數 {
    pub fn 代理地址(&self) -> Option<&str> {
        self.proxy.as_deref()
    }
    pub fn 倉庫域名(&self) -> Option<&str> {
        self.host.as_deref()
    }
    pub fn 令牌(&self) -> Option<&str> {
        self.token.as_deref()
    }
}

pub fn 下載配方包(
    衆配方: &[配方名片],
    下載場地: &Path,
    參數: &下載參數,
) -> anyhow::Result<()> {
    let 代理 = 參數.代理地址();
    for (包名, 一組配方包) in 配方包::按倉庫分組(衆配方, 參數.倉庫域名()) {
        let 首包 = 一組配方包.first().ok_or(anyhow!("至少應有一個配方包"))?;
        log::debug!("下載配方包: {}, 位於 {}", 包名, 首包.倉庫地址());
        let 本地倉庫 = 首包.本地路徑(下載場地);
        if 本地倉庫.exists() {
            同步既存倉庫(&本地倉庫, 代理)?;
        } else {
            搬運倉庫(首包, &本地倉庫, 代理)?;
        }
        // 驗證組內每個配方所請求的分支（或默認分支）均已拉取並可檢出
        for 包 in &一組配方包 {
            檢出倉庫分支(&本地倉庫, 包.倉庫分支())?;
        }
    }
    Ok(())
}

pub fn 檢出倉庫分支(本地路徑: &Path, 目標分支: Option<&str>) -> anyhow::Result<()> {
    if git2::Repository::open(本地路徑).is_err() {
        if let Some(分支) = 目標分支 {
            anyhow::bail!(
                "本地路徑不是有效的 Git 倉庫，無法檢出分支 {}: {}",
                分支,
                本地路徑.display()
            );
        }
        return Ok(());
    }
    git::checkout_branch(本地路徑, 目標分支)?;
    Ok(())
}

fn 搬運倉庫(包: &配方包, 本地路徑: &Path, 代理: Option<&str>) -> anyhow::Result<()> {
    let 網址 = &包.倉庫地址();
    git::clone(網址, 本地路徑, 代理)?;
    Ok(())
}

fn 同步既存倉庫(本地路徑: &Path, 代理: Option<&str>) -> anyhow::Result<()> {
    const 遠端代號: &str = "origin";
    git::pull(本地路徑, 遠端代號, 代理)?;
    Ok(())
}

mod git {
    use git2::build::{CheckoutBuilder, RepoBuilder};
    use git2::{
        AnnotatedCommit, AutotagOption, ErrorClass, ErrorCode, FetchOptions, Progress,
        ProxyOptions, Reference, Remote, RemoteCallbacks, Repository,
    };
    use indicatif::{ProgressBar, ProgressStyle};
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    fn update_progress_bar(state: &mut State) {
        if let Some(progress) = &state.progress {
            state.pb.set_length(progress.total_objects() as u64);
            state.pb.set_position(progress.received_objects() as u64);
        } else {
            state.pb.set_length(state.total as u64);
            state.pb.set_position(state.current as u64);
        }
        let message = if let Some(stats) = &state.progress {
            let network_pct = (100 * stats.received_objects()) / stats.total_objects();
            let index_pct = (100 * stats.indexed_objects()) / stats.total_objects();
            let co_pct = if state.total > 0 {
                (100 * state.current) / state.total
            } else {
                0
            };
            let kbytes = stats.received_bytes() / 1024;
            if stats.received_objects() == stats.total_objects() {
                format!(
                    "Resolving deltas {}/{}",
                    stats.indexed_deltas(),
                    stats.total_deltas()
                )
            } else {
                format!(
                    "net {}% ({} kb, {}/{}) / idx {}% ({}/{}) / chk {}% ({}/{}) {}",
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
        } else {
            String::new()
        };
        state.pb.set_message(message);
    }

    struct State {
        progress: Option<Progress<'static>>,
        total: usize,
        current: usize,
        path: Option<PathBuf>,
        pb: ProgressBar,
    }

    fn create_pbar_and_opts(
        proxy: Option<&str>,
    ) -> (ProgressBar, FetchOptions<'static>, CheckoutBuilder<'static>) {
        let pb = ProgressBar::new(0);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40}] [eta: {eta}]\n  {msg}")
                .unwrap()
                .progress_chars("█>-"),
        );
        let pb_clone = pb.clone();
        let state: &'static RefCell<State> = Box::leak(Box::new(RefCell::new(State {
            progress: None,
            total: 0,
            current: 0,
            path: None,
            pb,
        })));

        let mut cb = RemoteCallbacks::new();
        cb.transfer_progress(move |stats| {
            let mut state = state.borrow_mut();
            state.progress = Some(stats.to_owned());
            update_progress_bar(&mut state);
            true
        });

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(cb);
        if let Some(proxy_url) = proxy {
            let mut proxy_opts = ProxyOptions::new();
            proxy_opts.url(proxy_url);
            fo.proxy_options(proxy_opts);
        }

        let mut co = CheckoutBuilder::new();
        co.progress(move |path, cur, total| {
            let mut state = state.borrow_mut();
            state.path = path.map(|p| p.to_path_buf());
            state.current = cur;
            state.total = total;
            update_progress_bar(&mut state);
        });

        (pb_clone, fo, co)
    }

    pub fn clone(url: &str, path: &Path, proxy: Option<&str>) -> Result<(), git2::Error> {
        let (pb, fo, co) = create_pbar_and_opts(proxy);
        let mut builder = RepoBuilder::new();
        builder.fetch_options(fo).with_checkout(co);
        // 完整拉取所有遠端分支 (+refs/heads/*:refs/remotes/origin/*)，
        // 並由 libgit2 自動根據遠端 HEAD 建立 refs/remotes/origin/HEAD 指向遠端默認分支
        builder.clone(url, path)?;
        pb.finish();
        println!();

        Ok(())
    }

    fn do_fetch(
        refs: &[&str],
        remote: &mut Remote,
        proxy: Option<&str>,
    ) -> Result<(), git2::Error> {
        let (pb, mut fo, _co) = create_pbar_and_opts(proxy);
        // Always fetch all tags.
        // Perform a download and also update tips
        fo.download_tags(AutotagOption::All);
        println!("Fetching {} for repo", remote.name().unwrap_or("origin"));
        remote.fetch(refs, Some(&mut fo), None)?;
        pb.finish();

        // If there are local objects (we got a thin pack), then tell the user
        // how many objects we saved from having to cross the network.
        let stats = remote.stats();
        if stats.local_objects() > 0 {
            println!(
                "Received {}/{} objects in {} bytes (used {} local \
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

        Ok(())
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
        log::info!("{}", msg);
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
            log::debug!("Doing a fast forward on {}", remote_branch);
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
            // TODO: print the message and then checkout deteched head.
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
            log::debug!("分支 {} 已是最新提交，無需合併", remote_branch);
        }
        Ok(())
    }

    pub fn pull(
        repo_path: &Path,
        remote_name: &str,
        proxy: Option<&str>,
    ) -> Result<(), git2::Error> {
        let repo = Repository::open(repo_path)?;
        let mut remote = repo.find_remote(remote_name)?;
        // 傳入空切片 &[] 以使用 origin 配置的默認 refspec (+refs/heads/*:refs/remotes/origin/*)，
        // 一次拉取所有既有遠端分支的更新以及遠端後來新增的分支。
        do_fetch(&[], &mut remote, proxy)?;
        Ok(())
    }

    /// 解析代碼庫配置的默認分支名稱
    fn resolve_default_branch(repo: &Repository) -> Result<String, git2::Error> {
        // 1. 優先讀取克隆自遠端時記錄的 refs/remotes/origin/HEAD 符號引用
        if let Ok(origin_head) = repo.find_reference("refs/remotes/origin/HEAD") {
            if let Some(sym) = origin_head.symbolic_target() {
                if let Some(branch) = sym.strip_prefix("refs/remotes/origin/") {
                    return Ok(branch.to_string());
                }
            }
        }

        // 2. 若倉庫配置了 remote.origin.gh-resolved 或 branch.<name>.remote，或檢查 origin 下唯一的默認分支
        for candidate in ["main", "master"] {
            if repo
                .find_reference(&format!("refs/remotes/origin/{}", candidate))
                .is_ok()
            {
                return Ok(candidate.to_string());
            }
        }

        // 3. 本地初始化倉庫（無遠端 origin 時）檢查本地配置或本地分支
        if let Ok(cfg) = repo.config() {
            if let Ok(default_branch) = cfg.get_string("init.defaultBranch") {
                if repo
                    .find_reference(&format!("refs/heads/{}", default_branch))
                    .is_ok()
                {
                    return Ok(default_branch);
                }
            }
        }
        for candidate in ["main", "master"] {
            if repo
                .find_reference(&format!("refs/heads/{}", candidate))
                .is_ok()
            {
                return Ok(candidate.to_string());
            }
        }
        if let Ok(head) = repo.head() {
            if let Some(name) = head.shorthand() {
                return Ok(name.to_string());
            }
        }

        Err(git2::Error::new(
            ErrorCode::NotFound,
            ErrorClass::Reference,
            "無法確定代碼庫的默認分支",
        ))
    }

    pub fn checkout_branch(repo_path: &Path, branch: Option<&str>) -> Result<(), git2::Error> {
        let repo = Repository::open(repo_path)?;
        let target_branch = match branch {
            Some(b) => b.to_string(),
            None => resolve_default_branch(&repo)?,
        };

        let local_ref = format!("refs/heads/{}", target_branch);
        let remote_ref = format!("refs/remotes/origin/{}", target_branch);

        // 若遠端跟蹤分支存在（包含新克隆或剛 pull 拉取的新增分支），同步並快進本地分支
        if let Ok(remote_reference) = repo.find_reference(&remote_ref) {
            let fetch_commit = repo.reference_to_annotated_commit(&remote_reference)?;
            if repo.find_reference(&local_ref).is_ok() {
                let already_checked_out = repo
                    .head()
                    .ok()
                    .map(|h| {
                        h.name() == Some(local_ref.as_str())
                            && h.target() == Some(fetch_commit.id())
                    })
                    .unwrap_or(false);
                if !already_checked_out {
                    repo.set_head(&local_ref)?;
                    repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
                    do_merge(&repo, &target_branch, fetch_commit)?;
                }
            } else {
                repo.reference(
                    &local_ref,
                    fetch_commit.id(),
                    true,
                    &format!("Checkout remote branch {}", target_branch),
                )?;
                repo.set_head(&local_ref)?;
                repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
            }
        } else if repo.find_reference(&local_ref).is_ok() {
            // 無遠端跟蹤分支但存在本地分支（例如本地測試倉庫）
            repo.set_head(&local_ref)?;
            repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
        } else if let Ok(tag_ref) = repo.find_reference(&format!("refs/tags/{}", target_branch)) {
            let commit = tag_ref.peel_to_commit()?;
            repo.set_head_detached(commit.id())?;
            repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
        } else {
            return Err(git2::Error::new(
                ErrorCode::NotFound,
                ErrorClass::Reference,
                format!("在代碼庫中找不到目標分支或標籤: {}", target_branch),
            ));
        }

        // 校驗當前檢出的 HEAD 是否與目標分支匹配
        let head = repo.head()?;
        if !repo.head_detached()? && head.name() != Some(local_ref.as_str()) {
            return Err(git2::Error::new(
                ErrorCode::InvalidSpec,
                ErrorClass::Reference,
                format!(
                    "檢出分支後 HEAD ({:?}) 與目標分支 ({}) 不匹配",
                    head.name(),
                    local_ref
                ),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use std::fs;

    #[ignore]
    #[test]
    fn 測試搬運倉庫() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_dir = tempfile::tempdir()?;
        let 本地測試路徑 = tmp_dir.path().join("test-clone");
        搬運倉庫(
            &配方包 {
                配方: 配方名片 {
                    方家: "lotem".to_string(),
                    名字: "rime-cli".to_string(),
                    版本: None,
                },
                倉庫域名: None,
            },
            &本地測試路徑,
            None,
        )?;
        Ok(())
    }

    #[test]
    fn 測試克隆與拉取_全分支與默認分支及新增分支() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let 遠端路徑 = tmp_dir.path().join("upstream");
        let 本地路徑 = tmp_dir.path().join("local");
        fs::create_dir_all(&遠端路徑).unwrap();

        // 1. 建立模擬遠端倉庫，將默認分支設爲 "plum"（非 master），並同時建立 "hant" 分支
        let upstream = git2::Repository::init(&遠端路徑).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        fs::write(遠端路徑.join("default.yaml"), "v1\n").unwrap();
        let mut index = upstream.index().unwrap();
        index.add_path(Path::new("default.yaml")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = upstream.find_tree(tree_id).unwrap();
        let plum_commit_id = upstream
            .commit(Some("refs/heads/plum"), &sig, &sig, "init plum", &tree, &[])
            .unwrap();
        let plum_commit = upstream.find_commit(plum_commit_id).unwrap();
        upstream.set_head("refs/heads/plum").unwrap();

        // 在遠端建立 hant 分支
        upstream.branch("hant", &plum_commit, false).unwrap();
        upstream.set_head("refs/heads/hant").unwrap();
        fs::write(遠端路徑.join("hant.gram"), "hant-model\n").unwrap();
        let mut index = upstream.index().unwrap();
        index.add_path(Path::new("hant.gram")).unwrap();
        index.write().unwrap();
        let hant_tree_id = index.write_tree().unwrap();
        let hant_tree = upstream.find_tree(hant_tree_id).unwrap();
        upstream
            .commit(
                Some("refs/heads/hant"),
                &sig,
                &sig,
                "add hant",
                &hant_tree,
                &[&plum_commit],
            )
            .unwrap();
        // 將遠端 HEAD 設回配置的默認分支 refs/heads/plum
        upstream.set_head("refs/heads/plum").unwrap();

        // 2. 單次 clone，驗證同時拉取了所有分支並保留了遠端默認分支信息
        let 遠端網址 = 遠端路徑.to_str().unwrap();
        assert_ok!(git::clone(遠端網址, &本地路徑, None));
        assert_ok!(檢出倉庫分支(&本地路徑, Some("hant")));
        assert!(本地路徑.join("hant.gram").exists());

        // 未指定 @ 分支時，應自動檢出遠端配置的默認分支 "plum"
        assert_ok!(檢出倉庫分支(&本地路徑, None));
        assert_eq!(
            fs::read_to_string(本地路徑.join("default.yaml")).unwrap(),
            "v1\n"
        );
        assert!(!本地路徑.join("hant.gram").exists());

        // 3. 在遠端更新默認分支 "plum"，並新增一個此前不存在的 "hans" 分支
        upstream
            .checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .unwrap();
        fs::write(遠端路徑.join("default.yaml"), "v2\n").unwrap();
        let mut index = upstream.index().unwrap();
        index.add_path(Path::new("default.yaml")).unwrap();
        index.write().unwrap();
        let plum_v2_tree_id = index.write_tree().unwrap();
        let plum_v2_tree = upstream.find_tree(plum_v2_tree_id).unwrap();
        let plum_v2_commit_id = upstream
            .commit(
                Some("refs/heads/plum"),
                &sig,
                &sig,
                "update plum v2",
                &plum_v2_tree,
                &[&plum_commit],
            )
            .unwrap();
        let plum_v2_commit = upstream.find_commit(plum_v2_commit_id).unwrap();

        upstream.branch("hans", &plum_v2_commit, false).unwrap();
        upstream.set_head("refs/heads/hans").unwrap();
        fs::write(遠端路徑.join("hans.gram"), "hans-model\n").unwrap();
        let mut index = upstream.index().unwrap();
        index.add_path(Path::new("hans.gram")).unwrap();
        index.write().unwrap();
        let hans_tree_id = index.write_tree().unwrap();
        let hans_tree = upstream.find_tree(hans_tree_id).unwrap();
        upstream
            .commit(
                Some("refs/heads/hans"),
                &sig,
                &sig,
                "add hans",
                &hans_tree,
                &[&plum_v2_commit],
            )
            .unwrap();
        upstream.set_head("refs/heads/plum").unwrap();

        // 4. 執行單次同步既存倉庫 (git::pull)，驗證：
        //    (a) 默認分支 "plum" 快進至 v2
        //    (b) 遠端後來新增的 "hans" 分支成功拉取並可檢出
        //    (c) 不存在的分支返回錯誤
        assert_ok!(同步既存倉庫(&本地路徑, None));

        assert_ok!(檢出倉庫分支(&本地路徑, None));
        assert_eq!(
            fs::read_to_string(本地路徑.join("default.yaml")).unwrap(),
            "v2\n"
        );

        assert_ok!(檢出倉庫分支(&本地路徑, Some("hans")));
        assert!(本地路徑.join("hans.gram").exists());

        assert_err!(檢出倉庫分支(&本地路徑, Some("nonexistent-branch")));
    }
}
