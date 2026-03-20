mod app;
mod gui;
mod keymap;
mod misc;
mod opt;
mod persisted;
mod runtime;

pub(crate) use persisted::PersistedState;

use opt::Opt;
use structopt::StructOpt;

fn main() -> anyhow::Result<()> {
    app::run(Opt::from_args())
}
