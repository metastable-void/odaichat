
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

    GetCanvas {
        canvas_id: Arc<str>,
    },

    CanvasData {
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
        let tx2 = tx.clone();
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

                    Command::GetCanvas { canvas_id } => {
                        if let Some(d) = inner.get_canvas(&canvas_id) {
                            let _ = tx2.send(Command::CanvasData { canvas_id, png_bytes: d.into() });
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

    pub(crate) fn send_data_request(&self, canvas_id: &str) {
        self.cmd_sender.send(Command::GetCanvas { canvas_id: canvas_id.to_owned().into() }).ok();
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
            "INSERT OR REPLACE INTO canvas (canvas_id, canvas_data) VALUES (?1, ?2)",
            (canvas_id, png_bytes),
        )?;
        Ok(())
    }

    fn get_canvas(&self, canvas_id: &str) -> Option<Vec<u8>> {
        let db = &self.conn;
        db.query_row(
            "SELECT canvas_data FROM canvas WHERE canvas_id = ?1",
            [canvas_id],
            |row| row.get(0),
        ).ok()
    }
}
