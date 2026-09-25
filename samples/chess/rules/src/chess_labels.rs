//! Atomic piece names with color agreement for board accessibility.

use cozy_chess::{Color, Piece};
use trox::{LocalizedString, tx};

pub fn piece(color: Color, kind: Piece) -> LocalizedString {
  match (color, kind) {
    (Color::White, Piece::Pawn) => tx("White Pawn", "Chess piece and color."),
    (Color::White, Piece::Knight) => tx("White Knight", "Chess piece and color."),
    (Color::White, Piece::Bishop) => tx("White Bishop", "Chess piece and color."),
    (Color::White, Piece::Rook) => tx("White Rook", "Chess piece and color."),
    (Color::White, Piece::Queen) => tx("White Queen", "Chess piece and color."),
    (Color::White, Piece::King) => tx("White King", "Chess piece and color."),
    (Color::Black, Piece::Pawn) => tx("Black Pawn", "Chess piece and color."),
    (Color::Black, Piece::Knight) => tx("Black Knight", "Chess piece and color."),
    (Color::Black, Piece::Bishop) => tx("Black Bishop", "Chess piece and color."),
    (Color::Black, Piece::Rook) => tx("Black Rook", "Chess piece and color."),
    (Color::Black, Piece::Queen) => tx("Black Queen", "Chess piece and color."),
    (Color::Black, Piece::King) => tx("Black King", "Chess piece and color."),
  }
}

pub fn promotion(piece: Piece) -> LocalizedString {
  match piece {
    Piece::Queen => tx("Queen", "Promotion piece choice."),
    Piece::Rook => tx("Rook", "Promotion piece choice."),
    Piece::Bishop => tx("Bishop", "Promotion piece choice."),
    Piece::Knight => tx("Knight", "Promotion piece choice."),
    _ => panic!("invalid promotion piece"),
  }
}
