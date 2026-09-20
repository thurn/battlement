//! Logical chess position and stable presentation identities.

use battlement::ObjectId;
use cozy_chess::{Board, Color, File, Move, Piece, Rank, Square};

/// Stable object identity paired with its explicit board reference slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PieceIdentity {
  /// Host object identity retained while the piece moves.
  pub object_id: ObjectId,
  /// Slot in the board component's hook-backed reference table.
  pub reference_slot: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChessPiece {
  /// Presentation identity retained while the piece moves between squares.
  pub identity: PieceIdentity,
  /// Piece color.
  pub color: Color,
  /// Visible opaque prefab kind.
  pub kind: Piece,
}

#[derive(Clone)]
/// Logical board paired with the stable identities required by presentation.
///
/// `cozy_chess::Board` deliberately knows nothing about rendered entities. A
/// parallel square-indexed array lets rules remain authoritative while Reactant
/// can reconcile and animate the same object across moves.
pub struct ChessPosition {
  /// Authoritative chess position used for legality and terminal status.
  pub board: Board,
  pieces: [Option<ChessPiece>; 64],
}

/// Presentation-ready description of one accepted chess move.
///
/// Modeling special moves explicitly keeps animation code declarative: it does
/// not need to rediscover captures, castling, or promotion from a mutated board.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Movement {
  /// Ordinary translation to an empty square.
  Move {
    /// Stable identity of the moving piece.
    piece: PieceIdentity,
    /// Destination square.
    to: Square,
  },
  /// Knight translation split into two orthogonal animation legs.
  Knight {
    /// Stable identity of the moving knight.
    piece: PieceIdentity,
    /// Visual corner between the two legs.
    corner: Square,
    /// Destination square.
    to: Square,
  },
  /// Move that removes an opposing piece, including en passant.
  Capture {
    /// Stable identity of the moving piece.
    piece: PieceIdentity,
    /// Stable identity of the captured piece.
    captured: PieceIdentity,
    /// Square occupied by the captured piece before the move.
    capture_at: Square,
    /// Destination square of the moving piece.
    to: Square,
    /// Visual corner when the capturing piece is a knight.
    knight_corner: Option<Square>,
  },
  /// Coordinated king and rook translation.
  Castle {
    /// Stable identity of the king.
    king: PieceIdentity,
    /// King's visible destination.
    king_to: Square,
    /// Stable identity of the rook.
    rook: PieceIdentity,
    /// Rook's visible destination.
    rook_to: Square,
  },
  /// Pawn translation whose prefab changes when state commits.
  Promotion {
    /// Stable identity retained by the promoted piece.
    piece: PieceIdentity,
    /// Optional captured identity for a promotion capture.
    captured: Option<PieceIdentity>,
    /// Promotion square.
    to: Square,
  },
}

impl ChessPosition {
  /// Builds presentation identities for every occupied square on a board.
  ///
  /// Object IDs and reference slots are separate concerns: the former identifies
  /// host objects while the latter selects the matching hook-backed reference.
  pub fn from_board(board: Board, generation: u64) -> Self {
    let mut identities = fastrand::Rng::with_seed(generation ^ 0xA599_7560_9834_72D1);
    let pieces = std::array::from_fn(|index| {
      let square = Square::index(index);
      let object_id = piece_id(&mut identities);
      Some(ChessPiece {
        identity: PieceIdentity {
          object_id,
          reference_slot: index,
        },
        color: board.color_on(square)?,
        kind: board.piece_on(square)?,
      })
    });
    Self { board, pieces }
  }

  /// Returns the presentation identity currently occupying `square`.
  pub fn piece(&self, square: Square) -> Option<ChessPiece> {
    self.pieces[square as usize]
  }

  /// Finds a piece by the identity attached to its host object.
  pub fn piece_with_id(&self, object_id: ObjectId) -> Option<ChessPiece> {
    self
      .pieces
      .iter()
      .flatten()
      .find(|piece| piece.identity.object_id == object_id)
      .copied()
  }

  /// Returns unique player-visible targets for one source square.
  pub fn legal_destinations(&self, from: Square) -> Vec<Square> {
    let mut destinations = Vec::new();
    self.board.generate_moves_for(from.bitboard(), |moves| {
      destinations.extend(
        moves
          .into_iter()
          .map(|movement| visible_destination(&self.board, movement)),
      );
      false
    });
    destinations.sort_unstable();
    destinations.dedup();
    destinations
  }

  /// Returns all legal moves matching one visible source and destination.
  ///
  /// The result may contain several promotion choices because those share the
  /// same visible squares and are disambiguated by a typed prompt later.
  pub fn legal_moves(&self, from: Square, to: Square) -> Vec<Move> {
    if self.board.side_to_move() != Color::White || self.board.color_on(from) != Some(Color::White)
    {
      return Vec::new();
    }
    let mut candidates = Vec::new();
    self.board.generate_moves_for(from.bitboard(), |moves| {
      candidates.extend(
        moves
          .into_iter()
          .filter(|movement| visible_destination(&self.board, *movement) == to),
      );
      false
    });
    candidates
  }

  /// Applies one legal move to both the rules board and its identity map.
  ///
  /// Keeping these mutations together prevents rendered identity from drifting
  /// away from the accepted logical snapshot.
  pub fn apply(&mut self, movement: Move) {
    let color = self
      .board
      .color_on(movement.from)
      .expect("legal mover has a color");
    let piece = self
      .board
      .piece_on(movement.from)
      .expect("legal mover has a kind");
    if piece == Piece::King && self.board.color_on(movement.to) == Some(color) {
      self.apply_castle(movement, color);
    } else {
      let capture = capture_square(&self.board, movement, piece);
      self.pieces[capture as usize] = None;
      let mut moving = self.pieces[movement.from as usize]
        .take()
        .expect("legal mover has an identity");
      if let Some(promoted) = movement.promotion {
        moving.kind = promoted;
      }
      self.pieces[movement.to as usize] = Some(moving);
    }
    self.board.play_unchecked(movement);
  }

  /// Updates both identities for cozy-chess's king-to-rook castling encoding.
  fn apply_castle(&mut self, movement: Move, color: Color) {
    let (king_to, rook_to) = castle_destinations(movement, color);
    let king = self.pieces[movement.from as usize]
      .take()
      .expect("castling king has an identity");
    let rook = self.pieces[movement.to as usize]
      .take()
      .expect("castling rook has an identity");
    self.pieces[king_to as usize] = Some(king);
    self.pieces[rook_to as usize] = Some(rook);
  }

  /// Describes a legal move for presentation before mutating the board.
  ///
  /// Deriving this first is important: capture identities and the original
  /// squares needed by animation disappear once [`Self::apply`] commits them.
  pub fn movement(&self, movement: Move) -> Movement {
    let moving = self
      .piece(movement.from)
      .expect("legal mover has an identity");
    if moving.kind == Piece::King && self.board.color_on(movement.to) == Some(moving.color) {
      let (king_to, rook_to) = castle_destinations(movement, moving.color);
      return Movement::Castle {
        king: moving.identity,
        king_to,
        rook: self
          .piece(movement.to)
          .expect("castling rook has an identity")
          .identity,
        rook_to,
      };
    }
    let capture_at = capture_square(&self.board, movement, moving.kind);
    let captured = self.piece(capture_at).map(|piece| piece.identity);
    if movement.promotion.is_some() {
      return Movement::Promotion {
        piece: moving.identity,
        captured,
        to: movement.to,
      };
    }
    let knight_corner =
      (moving.kind == Piece::Knight).then(|| knight_corner(movement.from, movement.to));
    if let Some(captured) = captured {
      return Movement::Capture {
        piece: moving.identity,
        captured,
        capture_at,
        to: movement.to,
        knight_corner,
      };
    }
    if let Some(corner) = knight_corner {
      Movement::Knight {
        piece: moving.identity,
        corner,
        to: movement.to,
      }
    } else {
      Movement::Move {
        piece: moving.identity,
        to: movement.to,
      }
    }
  }
}

/// Generates an opaque, deterministic host identity for a piece slot.
fn piece_id(rng: &mut fastrand::Rng) -> ObjectId {
  let mut bytes = [0; 16];
  bytes[..8].copy_from_slice(&rng.u64(..).to_be_bytes());
  bytes[8..].copy_from_slice(&rng.u64(..).to_be_bytes());
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  ObjectId::from_bytes(bytes).expect("generated piece identity is nonzero")
}

/// Resolves cozy-chess's castling representation to the visible king target.
fn visible_destination(board: &Board, movement: Move) -> Square {
  let color = board.color_on(movement.from);
  if board.piece_on(movement.from) == Some(Piece::King) && board.color_on(movement.to) == color {
    castle_destinations(movement, color.expect("castling king has a color")).0
  } else {
    movement.to
  }
}

/// Locates the removed piece, accounting for en passant's empty destination.
fn capture_square(board: &Board, movement: Move, piece: Piece) -> Square {
  if piece == Piece::Pawn
    && movement.from.file() != movement.to.file()
    && board.piece_on(movement.to).is_none()
  {
    Square::new(movement.to.file(), movement.from.rank())
  } else {
    movement.to
  }
}

/// Converts cozy-chess's king-to-rook move into visible king and rook destinations.
fn castle_destinations(movement: Move, color: Color) -> (Square, Square) {
  let rank = if color == Color::White {
    Rank::First
  } else {
    Rank::Eighth
  };
  let short = movement.to.file() > movement.from.file();
  (
    Square::new(if short { File::G } else { File::C }, rank),
    Square::new(if short { File::F } else { File::D }, rank),
  )
}

/// Chooses the orthogonal corner that emphasizes the knight's longer first leg.
fn knight_corner(from: Square, to: Square) -> Square {
  if (from.file() as i8 - to.file() as i8).unsigned_abs()
    > (from.rank() as i8 - to.rank() as i8).unsigned_abs()
  {
    Square::new(to.file(), from.rank())
  } else {
    Square::new(from.file(), to.rank())
  }
}
