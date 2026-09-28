use crate::task_states::PetState;
pub type Frame = [[[u8; 3]; 6]; 6];
// Dim robot housing keeps the bright eyes distinct under the broad square glow.
pub fn frame_for(state: PetState) -> Frame {
    let rows = match state {
        PetState::Idle => [".dddd.", "dc..cd", "dc..cd", ".dddd.", "......", "..cc.."],
        PetState::Thinking => ["......", "......", "p.p.p.", "......", "......", "......"],
        PetState::Working => ["......", ".y..y.", "......", "......", ".y..y.", "......"],
        PetState::Waiting => ["..oo..", "..oo..", "..oo..", "......", "......", "..oo.."],
        PetState::Done => ["......", ".g..g.", "......", ".g..g.", "..gg..", "......"],
        PetState::Interrupted => ["......", ".r..r.", "..rr..", "..rr..", ".r..r.", "......"],
    };
    let mut frame = [[[0; 3]; 6]; 6];
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.bytes().enumerate() {
            frame[y][x] = match c {
                b'd' => [0, 7, 10],
                b'c' => [0, 100, 130],
                b'p' => [85, 45, 130],
                b'y' => [125, 95, 10],
                b'o' => [130, 60, 5],
                b'g' => [15, 125, 55],
                b'r' => [105, 25, 35],
                _ => [0, 0, 0],
            };
        }
    }
    frame
}
