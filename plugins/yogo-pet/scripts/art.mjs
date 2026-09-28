// Keep in sync with yogo-core/src/art.rs.
const palette={".": [0, 0, 0], "d": [0, 7, 10], "c": [0, 100, 130], "p": [85, 45, 130], "y": [125, 95, 10], "o": [130, 60, 5], "g": [15, 125, 55], "r": [105, 25, 35]};
const pictures={"idle": [".dddd.", "dc..cd", "dc..cd", ".dddd.", "......", "..cc.."], "thinking": ["......", "......", "p.p.p.", "......", "......", "......"], "working": ["......", ".y..y.", "......", "......", ".y..y.", "......"], "waiting": ["..oo..", "..oo..", "..oo..", "......", "......", "..oo.."], "done": ["......", ".g..g.", "......", ".g..g.", "..gg..", "......"], "interrupted": ["......", ".r..r.", "..rr..", "..rr..", ".r..r.", "......"]};
export const states=Object.keys(pictures);
export function frameFor(state){return (pictures[state]||pictures.idle).map(row=>[...row].map(c=>[...palette[c]]));}
