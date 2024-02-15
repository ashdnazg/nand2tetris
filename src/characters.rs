pub fn character_bitmaps(c: i16) -> [i16; 11] {
    match c {
        32 => [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        33 => [12, 30, 30, 30, 12, 12, 0, 12, 12, 0, 0],
        34 => [54, 54, 20, 0, 0, 0, 0, 0, 0, 0, 0],
        35 => [0, 18, 18, 63, 18, 18, 63, 18, 18, 0, 0],
        36 => [12, 30, 51, 3, 30, 48, 51, 30, 12, 12, 0],
        37 => [0, 0, 35, 51, 24, 12, 6, 51, 49, 0, 0],
        38 => [12, 30, 30, 12, 54, 27, 27, 27, 54, 0, 0],
        39 => [12, 12, 6, 0, 0, 0, 0, 0, 0, 0, 0],
        40 => [24, 12, 6, 6, 6, 6, 6, 12, 24, 0, 0],
        41 => [6, 12, 24, 24, 24, 24, 24, 12, 6, 0, 0],
        42 => [0, 0, 0, 51, 30, 63, 30, 51, 0, 0, 0],
        43 => [0, 0, 0, 12, 12, 63, 12, 12, 0, 0, 0],
        44 => [0, 0, 0, 0, 0, 0, 0, 12, 12, 6, 0],
        45 => [0, 0, 0, 0, 0, 63, 0, 0, 0, 0, 0],
        46 => [0, 0, 0, 0, 0, 0, 0, 12, 12, 0, 0],
        47 => [0, 0, 32, 48, 24, 12, 6, 3, 1, 0, 0],
        48 => [12, 30, 51, 51, 51, 51, 51, 30, 12, 0, 0],
        49 => [12, 14, 15, 12, 12, 12, 12, 12, 63, 0, 0],
        50 => [30, 51, 48, 24, 12, 6, 3, 51, 63, 0, 0],
        51 => [30, 51, 48, 48, 28, 48, 48, 51, 30, 0, 0],
        52 => [16, 24, 28, 26, 25, 63, 24, 24, 60, 0, 0],
        53 => [63, 3, 3, 31, 48, 48, 48, 51, 30, 0, 0],
        54 => [28, 6, 3, 3, 31, 51, 51, 51, 30, 0, 0],
        55 => [63, 49, 48, 48, 24, 12, 12, 12, 12, 0, 0],
        56 => [30, 51, 51, 51, 30, 51, 51, 51, 30, 0, 0],
        57 => [30, 51, 51, 51, 62, 48, 48, 24, 14, 0, 0],
        58 => [0, 0, 12, 12, 0, 0, 12, 12, 0, 0, 0],
        59 => [0, 0, 12, 12, 0, 0, 12, 12, 6, 0, 0],
        60 => [0, 0, 24, 12, 6, 3, 6, 12, 24, 0, 0],
        61 => [0, 0, 0, 63, 0, 0, 63, 0, 0, 0, 0],
        62 => [0, 0, 3, 6, 12, 24, 12, 6, 3, 0, 0],
        64 => [30, 51, 51, 59, 59, 59, 27, 3, 30, 0, 0],
        63 => [30, 51, 51, 24, 12, 12, 0, 12, 12, 0, 0],
        65 => [12, 30, 51, 51, 63, 51, 51, 51, 51, 0, 0],
        66 => [31, 51, 51, 51, 31, 51, 51, 51, 31, 0, 0],
        67 => [28, 54, 35, 3, 3, 3, 35, 54, 28, 0, 0],
        68 => [15, 27, 51, 51, 51, 51, 51, 27, 15, 0, 0],
        69 => [63, 51, 35, 11, 15, 11, 35, 51, 63, 0, 0],
        70 => [63, 51, 35, 11, 15, 11, 3, 3, 3, 0, 0],
        71 => [28, 54, 35, 3, 59, 51, 51, 54, 44, 0, 0],
        72 => [51, 51, 51, 51, 63, 51, 51, 51, 51, 0, 0],
        73 => [30, 12, 12, 12, 12, 12, 12, 12, 30, 0, 0],
        74 => [60, 24, 24, 24, 24, 24, 27, 27, 14, 0, 0],
        75 => [51, 51, 51, 27, 15, 27, 51, 51, 51, 0, 0],
        76 => [3, 3, 3, 3, 3, 3, 35, 51, 63, 0, 0],
        77 => [33, 51, 63, 63, 51, 51, 51, 51, 51, 0, 0],
        78 => [51, 51, 55, 55, 63, 59, 59, 51, 51, 0, 0],
        79 => [30, 51, 51, 51, 51, 51, 51, 51, 30, 0, 0],
        80 => [31, 51, 51, 51, 31, 3, 3, 3, 3, 0, 0],
        81 => [30, 51, 51, 51, 51, 51, 63, 59, 30, 48, 0],
        82 => [31, 51, 51, 51, 31, 27, 51, 51, 51, 0, 0],
        83 => [30, 51, 51, 6, 28, 48, 51, 51, 30, 0, 0],
        84 => [63, 63, 45, 12, 12, 12, 12, 12, 30, 0, 0],
        85 => [51, 51, 51, 51, 51, 51, 51, 51, 30, 0, 0],
        86 => [51, 51, 51, 51, 51, 30, 30, 12, 12, 0, 0],
        87 => [51, 51, 51, 51, 51, 63, 63, 63, 18, 0, 0],
        88 => [51, 51, 30, 30, 12, 30, 30, 51, 51, 0, 0],
        89 => [51, 51, 51, 51, 30, 12, 12, 12, 30, 0, 0],
        90 => [63, 51, 49, 24, 12, 6, 35, 51, 63, 0, 0],
        91 => [30, 6, 6, 6, 6, 6, 6, 6, 30, 0, 0],
        92 => [0, 0, 1, 3, 6, 12, 24, 48, 32, 0, 0],
        93 => [30, 24, 24, 24, 24, 24, 24, 24, 30, 0, 0],
        94 => [8, 28, 54, 0, 0, 0, 0, 0, 0, 0, 0],
        95 => [0, 0, 0, 0, 0, 0, 0, 0, 0, 63, 0],
        96 => [6, 12, 24, 0, 0, 0, 0, 0, 0, 0, 0],
        97 => [0, 0, 0, 14, 24, 30, 27, 27, 54, 0, 0],
        98 => [3, 3, 3, 15, 27, 51, 51, 51, 30, 0, 0],
        99 => [0, 0, 0, 30, 51, 3, 3, 51, 30, 0, 0],
        100 => [48, 48, 48, 60, 54, 51, 51, 51, 30, 0, 0],
        101 => [0, 0, 0, 30, 51, 63, 3, 51, 30, 0, 0],
        102 => [28, 54, 38, 6, 15, 6, 6, 6, 15, 0, 0],
        103 => [0, 0, 30, 51, 51, 51, 62, 48, 51, 30, 0],
        104 => [3, 3, 3, 27, 55, 51, 51, 51, 51, 0, 0],
        105 => [12, 12, 0, 14, 12, 12, 12, 12, 30, 0, 0],
        106 => [48, 48, 0, 56, 48, 48, 48, 48, 51, 30, 0],
        107 => [3, 3, 3, 51, 27, 15, 15, 27, 51, 0, 0],
        108 => [14, 12, 12, 12, 12, 12, 12, 12, 30, 0, 0],
        109 => [0, 0, 0, 29, 63, 43, 43, 43, 43, 0, 0],
        110 => [0, 0, 0, 29, 51, 51, 51, 51, 51, 0, 0],
        111 => [0, 0, 0, 30, 51, 51, 51, 51, 30, 0, 0],
        112 => [0, 0, 0, 30, 51, 51, 51, 31, 3, 3, 0],
        113 => [0, 0, 0, 30, 51, 51, 51, 62, 48, 48, 0],
        114 => [0, 0, 0, 29, 55, 51, 3, 3, 7, 0, 0],
        115 => [0, 0, 0, 30, 51, 6, 24, 51, 30, 0, 0],
        116 => [4, 6, 6, 15, 6, 6, 6, 54, 28, 0, 0],
        117 => [0, 0, 0, 27, 27, 27, 27, 27, 54, 0, 0],
        118 => [0, 0, 0, 51, 51, 51, 51, 30, 12, 0, 0],
        119 => [0, 0, 0, 51, 51, 51, 63, 63, 18, 0, 0],
        120 => [0, 0, 0, 51, 30, 12, 12, 30, 51, 0, 0],
        121 => [0, 0, 0, 51, 51, 51, 62, 48, 24, 15, 0],
        122 => [0, 0, 0, 63, 27, 12, 6, 51, 63, 0, 0],
        123 => [56, 12, 12, 12, 7, 12, 12, 12, 56, 0, 0],
        124 => [12, 12, 12, 12, 12, 12, 12, 12, 12, 0, 0],
        125 => [7, 12, 12, 12, 56, 12, 12, 12, 7, 0, 0],
        126 => [38, 45, 25, 0, 0, 0, 0, 0, 0, 0, 0],
        _ => [63, 63, 63, 63, 63, 63, 63, 63, 63, 0, 0],
    }
}