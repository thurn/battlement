//! Logical starting positions: no JSON, FEN parser, disk access, or presentation IDs.
//! BoardBuilder validates kings, legality, castling rights, and en-passant metadata.
use cozy_chess::{Board, BoardBuilder, Color, File, Piece, Square};

pub fn position(pieces: &[(Square, Color, Piece)]) -> BoardBuilder {
  let mut b = BoardBuilder::empty();
  for &(square, color, piece) in pieces {
    *b.square_mut(square) = Some((piece, color));
  }
  b
}

pub fn initial() -> Board {
  Board::default()
}

pub fn capture() -> Board {
  let mut b = kings();
  *b.square_mut(Square::D4) = Some((Piece::Bishop, Color::White));
  *b.square_mut(Square::E5) = Some((Piece::Pawn, Color::Black));
  b.build().unwrap()
}

pub fn knight_capture() -> Board {
  let mut b = kings();
  *b.square_mut(Square::D4) = Some((Piece::Knight, Color::White));
  *b.square_mut(Square::E6) = Some((Piece::Pawn, Color::Black));
  b.build().unwrap()
}

pub fn castle() -> Board {
  let mut b = kings();
  *b.square_mut(Square::A1) = Some((Piece::Rook, Color::White));
  *b.square_mut(Square::H1) = Some((Piece::Rook, Color::White));
  b.castle_rights[Color::White as usize].short = Some(File::H);
  b.castle_rights[Color::White as usize].long = Some(File::A);
  b.build().unwrap()
}

pub fn en_passant() -> Board {
  let mut b = kings();
  *b.square_mut(Square::E5) = Some((Piece::Pawn, Color::White));
  *b.square_mut(Square::D5) = Some((Piece::Pawn, Color::Black));
  b.en_passant = Some(Square::D6);
  b.build().unwrap()
}

pub fn promotion() -> Board {
  let mut b = kings();
  *b.square_mut(Square::A7) = Some((Piece::Pawn, Color::White));
  *b.square_mut(Square::B8) = Some((Piece::Rook, Color::Black));
  b.build().unwrap()
}

pub fn check() -> Board {
  let mut b = kings();
  *b.square_mut(Square::A2) = Some((Piece::Rook, Color::White));
  b.build().unwrap()
}

pub fn player_win() -> Board {
  position(&[
    (Square::F7, Color::White, Piece::King),
    (Square::G6, Color::White, Piece::Queen),
    (Square::H8, Color::Black, Piece::King),
  ])
  .build()
  .unwrap()
}

pub fn draw() -> Board {
  position(&[
    (Square::C6, Color::White, Piece::King),
    (Square::C7, Color::White, Piece::Queen),
    (Square::A8, Color::Black, Piece::King),
  ])
  .build()
  .unwrap()
}

pub fn computer_win() -> Board {
  let mut b = position(&[
    (Square::H1, Color::White, Piece::King),
    (Square::G3, Color::Black, Piece::Queen),
    (Square::F3, Color::Black, Piece::King),
  ]);
  b.side_to_move = Color::Black;
  b.build().unwrap()
}

/// Independently specified expected board after e2-e4 and the zero-budget a7-a5 reply.
pub fn opening_reply() -> Board {
  let mut board = BoardBuilder::from_board(&Board::default());
  *board.square_mut(Square::E2) = None;
  *board.square_mut(Square::E4) = Some((Piece::Pawn, Color::White));
  *board.square_mut(Square::A7) = None;
  *board.square_mut(Square::A5) = Some((Piece::Pawn, Color::Black));
  board.build().unwrap()
}

fn kings() -> BoardBuilder {
  position(&[
    (Square::E1, Color::White, Piece::King),
    (Square::E8, Color::Black, Piece::King),
  ])
}
