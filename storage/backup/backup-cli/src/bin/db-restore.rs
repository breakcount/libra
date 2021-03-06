// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use backup_cli::{
    backup_types::state_snapshot::restore::{
        StateSnapshotRestoreController, StateSnapshotRestoreOpt,
    },
    storage::StorageOpt,
    utils::GlobalRestoreOpt,
};
use libradb::LibraDB;
use std::sync::Arc;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    #[structopt(flatten)]
    global: GlobalRestoreOpt,

    #[structopt(flatten)]
    state_snapshot: StateSnapshotRestoreOpt,

    #[structopt(subcommand)]
    storage: StorageOpt,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();

    let db = Arc::new(
        LibraDB::open(
            opt.global.db_dir,
            false, /* read_only */
            None,  /* pruner */
        )
        .expect("Failed opening DB."),
    );
    let storage = opt.storage.init_storage().await?;
    let restore_handler = Arc::new(db.get_restore_handler());
    StateSnapshotRestoreController::new(opt.state_snapshot, storage, restore_handler)
        .run()
        .await
        .context("Failed restoring state_snapshot.")?;

    println!("Finished restoring account state.");

    Ok(())
}
