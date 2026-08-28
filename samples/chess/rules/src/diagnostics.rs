use battlement::{
  Batch, BatchId, Command, Connect, ParallelCommandGroup, Response, ResponseMessage, SessionId,
};
use battlement_cloud::diagnostics::{DiagnosticsCommand, DiagnosticsMetadata};
use cozy_chess::{Board, GameStatus};

const MODULE_ID: &str = "battlement.diagnostics";

pub(crate) fn is_available(connect: &Connect) -> bool {
  connect.modules.iter().any(|module| module == MODULE_ID)
}

pub(crate) fn record_session_started(
  response: &mut Response,
  enabled: bool,
  session_id: SessionId,
  resumed: bool,
  board: &Board,
) {
  if !enabled {
    return;
  }
  self::append_commands(
    response,
    session_id,
    [
      self::metadata_command("sample.name", "chess"),
      self::metadata_command("sample.rules_version", env!("CARGO_PKG_VERSION")),
      self::metadata_command("chess.opponent", "computer"),
      self::metadata_command("chess.game_origin", if resumed { "saved" } else { "new" }),
      self::metadata_command("chess.game_status", self::status_name(board.status())),
    ],
  );
}

pub(crate) fn record_game_started(response: &mut Response, enabled: bool, session_id: SessionId) {
  if !enabled {
    return;
  }
  self::append_commands(
    response,
    session_id,
    [
      self::metadata_command("chess.game_origin", "new"),
      self::metadata_command("chess.game_status", "ongoing"),
    ],
  );
}

pub(crate) fn record_game_status(
  response: &mut Response,
  enabled: bool,
  session_id: SessionId,
  status: GameStatus,
) {
  if !enabled || status == GameStatus::Ongoing {
    return;
  }
  self::append_commands(
    response,
    session_id,
    [self::metadata_command(
      "chess.game_status",
      self::status_name(status),
    )],
  );
}

fn append_commands(
  response: &mut Response,
  session_id: SessionId,
  commands: impl IntoIterator<Item = DiagnosticsCommand>,
) {
  response.messages.push(ResponseMessage::Batch(Batch::new(
    BatchId::new_v4(),
    session_id,
    commands
      .into_iter()
      .map(|command| ParallelCommandGroup::new(vec![Command::diagnostics(command)]))
      .collect(),
  )));
}

fn metadata_command(key: &str, value: &str) -> DiagnosticsCommand {
  DiagnosticsCommand::SetMetadata(
    DiagnosticsMetadata::set(key, value).expect("sample Diagnostics metadata is valid"),
  )
}

const fn status_name(status: GameStatus) -> &'static str {
  match status {
    GameStatus::Ongoing => "ongoing",
    GameStatus::Drawn => "drawn",
    GameStatus::Won => "won",
  }
}
