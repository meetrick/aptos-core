// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_backup_cli::{
    metadata::{
        cache::{sync_and_load, MetadataCacheOpt},
        StateSnapshotBackupMeta,
    },
    storage::DBToolStorageOpt,
    utils::ConcurrentDownloadsOpt,
};
use aptos_logger::warn;
use aptos_types::transaction::Version;
use clap::Parser;
use itertools::Itertools;
use std::{io::Write, iter::once, path::PathBuf};

#[derive(Parser)]
pub struct Opt {
    #[clap(flatten)]
    metadata_cache_opt: MetadataCacheOpt,
    #[clap(flatten)]
    storage: DBToolStorageOpt,
    #[clap(flatten)]
    concurrent_downloads: ConcurrentDownloadsOpt,
    #[clap(
        long,
        help = "The first transaction version required to be replayed and verified. [Defaults to 0]"
    )]
    start_version: Option<Version>,
    #[clap(
        long,
        help = "Target number of transactions for each job to replay",
        default_value = "10000000"
    )]
    target_job_size: u64,
    #[clap(
        long,
        help = "Determines the oldest epoch to replay, relative to the latest",
        default_value = "4000"
    )]
    max_epochs: u64,
    #[clap(long, help = "Output job ranges")]
    output_json_file: PathBuf,
}

impl Opt {
    pub async fn run(self) -> anyhow::Result<()> {
        let storage = self.storage.init_storage().await?;
        let metadata_view = sync_and_load(
            &self.metadata_cache_opt,
            storage,
            self.concurrent_downloads.get(),
        )
        .await?;

        let storage_state = metadata_view.get_storage_state()?;
        let global_end_version = storage_state
            .latest_transaction_version
            .expect("No transaction backups.")
            + 1;
        let latest_epoch = storage_state
            .latest_state_snapshot_epoch
            .expect("No state snapshots.");
        let max_epochs = self.max_epochs.min(latest_epoch + 1);
        let global_min_epoch = latest_epoch + 1 - max_epochs;

        let fake_end = StateSnapshotBackupMeta {
            epoch: latest_epoch,
            version: global_end_version,
            manifest: "".to_string(),
        };
        let mut job_idx = 0;
        let job_ranges = metadata_view
            .all_state_snapshots()
            .iter()
            .filter(|s| s.epoch >= global_min_epoch && s.version <= global_end_version)
            .chain(once(&fake_end))
            .collect_vec()
            .iter()
            .rev()
            .tuple_windows()
            // to simplify things, if start_version appears in the middle of a range, give up the range
            .take_while(|(_end, begin)| begin.version >= self.start_version.unwrap_or(0))
            .peekable()
            .batching(|it| {
                job_idx += 1;
                match it.next() {
                    Some((end, mut begin)) => {
                        if end.version - begin.version >= self.target_job_size {
                            // cut big range short, this hopefully automatically skips load tests
                            let msg = if end.epoch - begin.epoch > 15 {
                                "!!! Need more snapshots !!!"
                            } else {
                                ""
                            };
                            warn!(
                                begin = begin,
                                end = end,
                                "Big gap between snapshots. {} versions in {} epochs. {}",
                                end.version - begin.version,
                                end.epoch - begin.epoch,
                                msg,
                            );
                            Some((
                                format!("{job_idx}-Partial"),
                                begin.version,
                                begin.version + self.target_job_size,
                                format!(
                                    "Partial replay epoch {} - {}, {} txns starting from version {}, another {} versions omitted, until {}. {}",
                                    begin.epoch,
                                    end.epoch - 1,
                                    self.target_job_size,
                                    begin.version,
                                    end.version - begin.version - self.target_job_size,
                                    end.version,
                                    msg
                                )
                            ))
                        } else {
                            while let Some((_prev_end, prev_begin)) = it.peek() {
                                if end.version - prev_begin.version > self.target_job_size {
                                    break;
                                }
                                begin = prev_begin;
                                let _ = it.next();
                            }
                            Some((
                                format!("{job_idx}"),
                                begin.version,
                                end.version,
                                format!(
                                    "Replay epoch {} - {}, {} txns starting from version {}.",
                                    begin.epoch,
                                    end.epoch - 1,
                                    end.version - begin.version,
                                    begin.version,
                                )
                            ))
                        }
                    },
                    None => None,
                }
            })
            .map(|(name, begin, end, desc)| format!("{name} {begin} {end} {desc}"))
            .collect_vec();

        std::fs::File::create(&self.output_json_file)?
            .write_all(&serde_json::to_vec(&job_ranges)?)?;

        Ok(())
    }
}