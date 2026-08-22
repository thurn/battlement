use std::{
    cmp::Reverse,
    time::{Duration, Instant},
};

use cozy_chess::{Board, Color, GameStatus, Move, Piece};
use rayon::prelude::*;

const INFINITY: i32 = 1_000_000;
const CHECKMATE: i32 = 100_000;
const PIECE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 0];
/// Searches for the best move found before the think-time budget expires.
pub(super) fn choose_move(board: &Board, think_time: Duration) -> Option<Move> {
    self::search(board, think_time)
}

fn search(board: &Board, think_time: Duration) -> Option<Move> {
    let deadline = Instant::now() + think_time;
    let mut moves = self::legal_moves(board);
    let mut best = *moves.first()?;
    if think_time.is_zero() {
        return Some(best);
    }

    for depth in 1..=64 {
        self::prioritize(&mut moves, best);
        let scores = moves
            .par_iter()
            .map(|&mv| {
                let mut child = board.clone();
                child.play_unchecked(mv);
                self::negamax(&child, depth - 1, -INFINITY, INFINITY, 1, deadline)
                    .map(|score| (score.saturating_neg(), mv))
            })
            .collect::<Vec<_>>();
        if scores.iter().any(Option::is_none) {
            break;
        }
        best = scores
            .into_iter()
            .flatten()
            .max_by_key(|&(score, _)| score)
            .expect("a legal position has a root move")
            .1;
        if Instant::now() >= deadline {
            break;
        }
    }
    Some(best)
}

fn negamax(
    board: &Board,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    ply: i32,
    deadline: Instant,
) -> Option<i32> {
    if Instant::now() >= deadline {
        return None;
    }
    match board.status() {
        GameStatus::Won => return Some(-CHECKMATE + ply),
        GameStatus::Drawn => return Some(0),
        GameStatus::Ongoing => {}
    }
    if depth == 0 {
        return self::quiescence(board, alpha, beta, ply, deadline);
    }

    let mut moves = self::legal_moves(board);
    self::order_moves(board, &mut moves);
    for mv in moves {
        let mut child = board.clone();
        child.play_unchecked(mv);
        let score =
            self::negamax(&child, depth - 1, -beta, -alpha, ply + 1, deadline)?.saturating_neg();
        if score >= beta {
            return Some(beta);
        }
        alpha = alpha.max(score);
    }
    Some(alpha)
}

fn quiescence(
    board: &Board,
    mut alpha: i32,
    beta: i32,
    ply: i32,
    deadline: Instant,
) -> Option<i32> {
    if Instant::now() >= deadline {
        return None;
    }
    if board.status() != GameStatus::Ongoing {
        return Some(if board.status() == GameStatus::Won {
            -CHECKMATE + ply
        } else {
            0
        });
    }
    let in_check = !board.checkers().is_empty();
    if !in_check {
        alpha = alpha.max(self::evaluate(board));
        if alpha >= beta {
            return Some(beta);
        }
    }

    let mut moves = self::legal_moves(board)
        .into_iter()
        .filter(|&mv| in_check || self::is_tactical(board, mv))
        .collect::<Vec<_>>();
    self::order_moves(board, &mut moves);
    for mv in moves {
        let mut child = board.clone();
        child.play_unchecked(mv);
        let score = self::quiescence(&child, -beta, -alpha, ply + 1, deadline)?.saturating_neg();
        if score >= beta {
            return Some(beta);
        }
        alpha = alpha.max(score);
    }
    Some(alpha)
}

fn evaluate(board: &Board) -> i32 {
    let mut score = 0;
    for color in Color::ALL {
        let sign = if color == Color::White { 1 } else { -1 };
        for piece in Piece::ALL {
            for square in board.colored_pieces(color, piece) {
                let relative = square.relative_to(color);
                let file = relative.file() as i32;
                let rank = relative.rank() as i32;
                let center = 6 - (file - 3).abs() - (rank - 3).abs();
                let activity = match piece {
                    Piece::Pawn => rank * 7 + center * 2,
                    Piece::Knight => center * 9,
                    Piece::Bishop => center * 5,
                    Piece::Rook => rank * 2,
                    Piece::Queen => center * 2,
                    Piece::King => -center * 3,
                };
                score += sign * (PIECE_VALUES[piece as usize] + activity);
            }
        }
        if board.colored_pieces(color, Piece::Bishop).len() >= 2 {
            score += sign * 25;
        }
    }
    if board.side_to_move() == Color::White {
        score
    } else {
        -score
    }
}

fn legal_moves(board: &Board) -> Vec<Move> {
    let mut moves = Vec::new();
    board.generate_moves(|piece_moves| {
        moves.extend(piece_moves);
        false
    });
    moves
}

fn order_moves(board: &Board, moves: &mut [Move]) {
    moves.sort_unstable_by_key(|&mv| Reverse(self::move_score(board, mv)));
}

fn move_score(board: &Board, mv: Move) -> i32 {
    let attacker = board.piece_on(mv.from).expect("legal moves have a piece");
    let victim = board.piece_on(mv.to).unwrap_or(Piece::Pawn);
    let capture = if self::is_tactical(board, mv) {
        10 * PIECE_VALUES[victim as usize] - PIECE_VALUES[attacker as usize]
    } else {
        0
    };
    capture
        + mv.promotion
            .map_or(0, |piece| PIECE_VALUES[piece as usize] + 800)
}

fn is_tactical(board: &Board, mv: Move) -> bool {
    mv.promotion.is_some()
        || board
            .color_on(mv.to)
            .is_some_and(|color| color != board.side_to_move())
        || (board.piece_on(mv.from) == Some(Piece::Pawn)
            && mv.from.file() != mv.to.file()
            && board.piece_on(mv.to).is_none())
}

fn prioritize(moves: &mut [Move], best: Move) {
    if let Some(index) = moves.iter().position(|&mv| mv == best) {
        moves.swap(0, index);
    }
}
