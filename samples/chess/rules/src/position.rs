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
pub(crate) struct ChessPosition {
  pub(crate) board: Board,
  pieces: [Option<ChessPiece>; 64],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Movement {
  Move {
    piece: ObjectId,
    to: Square,
  },
  Knight {
    piece: ObjectId,
    corner: Square,
    to: Square,
  },
  Capture {
    piece: ObjectId,
    captured: ObjectId,
    capture_at: Square,
    to: Square,
    knight_corner: Option<Square>,
  },
  Castle {
    king: ObjectId,
    king_to: Square,
    rook: ObjectId,
    rook_to: Square,
  },
  Promotion {
    piece: ObjectId,
    captured: Option<ObjectId>,
    to: Square,
  },
}

impl ChessPosition {
  pub(crate) fn from_board(board: Board, generation: u32) -> Self {
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

  pub(crate) fn piece(&self, square: Square) -> Option<ChessPiece> {
    self.pieces[square as usize]
  }

  pub(crate) fn legal_moves(&self, from: Square, to: Square) -> Vec<Move> {
    crate::player_moves(&self.board, from, to)
  }

  pub(crate) fn apply(&mut self, movement: Move) {
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

  pub(crate) fn movement(&self, movement: Move) -> Movement {
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

fn knight_corner(from: Square, to: Square) -> Square {
  if (from.file() as i8 - to.file() as i8).unsigned_abs()
    > (from.rank() as i8 - to.rank() as i8).unsigned_abs()
  {
    Square::new(to.file(), from.rank())
  } else {
    Square::new(from.file(), to.rank())
  }
}
