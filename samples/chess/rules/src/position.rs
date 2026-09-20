//! Logical chess position and stable presentation identities.

use battlement::ObjectId;
use cozy_chess::{Board, Color, File, Move, Piece, Rank, Square};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChessPiece {
  /// Stable entity identity retained while the piece moves between squares.
  pub entity_id: ObjectId,
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
    piece: ObjectId,
    /// Destination square.
    to: Square,
  },
  /// Knight translation split into two orthogonal animation legs.
  Knight {
    /// Stable identity of the moving knight.
    piece: ObjectId,
    /// Visual corner between the two legs.
    corner: Square,
    /// Destination square.
    to: Square,
  },
  /// Move that removes an opposing piece, including en passant.
  Capture {
    /// Stable identity of the moving piece.
    piece: ObjectId,
    /// Stable identity of the captured piece.
    captured: ObjectId,
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
    king: ObjectId,
    /// King's visible destination.
    king_to: Square,
    /// Stable identity of the rook.
    rook: ObjectId,
    /// Rook's visible destination.
    rook_to: Square,
  },
  /// Pawn translation whose prefab changes when state commits.
  Promotion {
    /// Stable identity retained by the promoted piece.
    piece: ObjectId,
    /// Optional captured identity for a promotion capture.
    captured: Option<ObjectId>,
    /// Promotion square.
    to: Square,
  },
}

impl ChessPosition {
  /// Builds presentation identities for every occupied square on a board.
  ///
  /// A session generation is encoded into each ID so replacing the game remounts
  /// pieces, while moves within one game preserve identity for reconciliation.
  pub fn from_board(board: Board, generation: u32) -> Self {
    let pieces = std::array::from_fn(|index| {
      let square = Square::index(index);
      Some(ChessPiece {
        entity_id: crate::piece_entity_id(index, generation),
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

  /// Returns all legal moves matching one visible source and destination.
  ///
  /// The result may contain several promotion choices because those share the
  /// same visible squares and are disambiguated by a typed prompt later.
  pub fn legal_moves(&self, from: Square, to: Square) -> Vec<Move> {
    crate::player_moves(&self.board, from, to)
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
        king: moving.entity_id,
        king_to,
        rook: self
          .piece(movement.to)
          .expect("castling rook has an identity")
          .entity_id,
        rook_to,
      };
    }
    let capture_at = capture_square(&self.board, movement, moving.kind);
    let captured = self.piece(capture_at).map(|piece| piece.entity_id);
    if movement.promotion.is_some() {
      return Movement::Promotion {
        piece: moving.entity_id,
        captured,
        to: movement.to,
      };
    }
    let knight_corner =
      (moving.kind == Piece::Knight).then(|| knight_corner(movement.from, movement.to));
    if let Some(captured) = captured {
      return Movement::Capture {
        piece: moving.entity_id,
        captured,
        capture_at,
        to: movement.to,
        knight_corner,
      };
    }
    if let Some(corner) = knight_corner {
      Movement::Knight {
        piece: moving.entity_id,
        corner,
        to: movement.to,
      }
    } else {
      Movement::Move {
        piece: moving.entity_id,
        to: movement.to,
      }
    }
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
