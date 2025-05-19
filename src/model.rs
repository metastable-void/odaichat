
use std::{path::Path, sync::Arc};
use rusqlite::Connection;
use tokio::sync::broadcast;

static SCHEMA: &str = include_str!("./schema.sql");

#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) enum Command {
    UpdateCanvas {
        canvas_id: Arc<str>,
        png_bytes: Arc<[u8]>,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct OdaiChat {
    cmd_sender: broadcast::Sender<Command>,
}

impl OdaiChat {
    pub(crate) fn open<P: AsRef<Path>>(db_path: P) -> Result<Self, anyhow::Error> {
        let (tx, mut rx) = broadcast::channel(16);
        let inner = OdaiChatInner::open(db_path)?;
        std::thread::spawn(move || {
            loop {
                let cmd = rx.blocking_recv().unwrap();

                #[allow(unreachable_patterns)]
                match cmd {
                    Command::UpdateCanvas { canvas_id, png_bytes } => {
                        if let Err(e) = inner.update_canvas(&canvas_id, &png_bytes) {
                            log::error!("Error: {}", e);
                        }
                    },

                    _ => {},
                }
            }
        });

        Ok(Self {
            cmd_sender: tx,
        })
    }

    pub(crate) fn send_command(&self, cmd: Command) {
        let _ = self.cmd_sender.send(cmd);
    }

    pub(crate) fn get_command_receiver(&self) -> broadcast::Receiver<Command> {
        self.cmd_sender.subscribe()
    }
}

#[derive(Debug)]
struct OdaiChatInner {
    conn: Connection,
}

impl OdaiChatInner {
    fn open<P: AsRef<Path>>(db_path: P) -> Result<Self, anyhow::Error> {
        let db = Connection::open(&db_path)?;
        db.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: db
        })
    }

    // this does not send updates to other clients.
    // please make sure that you do that yourself.
    // for a single server, in-process broadcast is sufficient,
    // but for a cluster, please consider something like Redis.
    fn update_canvas(&self, canvas_id: &str, png_bytes: &[u8]) -> Result<(), anyhow::Error> {
        let db = &self.conn;
        db.execute(
            "INSERT INTO canvas (canvas_id, canvas_data) VALUES (?1, ?2) ON CONFLICT UPDATE",
            (canvas_id, png_bytes),
        )?;
        Ok(())
    }
}
