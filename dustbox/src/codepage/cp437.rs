pub fn to_utf8(v: &[u8]) -> String {
    let mut s = String::new();
    for b in v {
        s.push(u8_as_char(*b));
    }
    s
}

/// converts byte to a symbol in code page 437 ("extended ASCII"), presented as a utf8 char
/// https://en.wikipedia.org/wiki/Code_page_437
pub fn u8_as_char(b: u8) -> char {
    match b {
        0x00 => 0 as char, // 0000 - NUL
        0x01 => '☺', // 263A
        0x02 => '☻', // 263B
        0x03 => '♥', // 2665
        0x04 => '♦', // 2666
        0x05 => '♣', // 2663 - ENQUIRY
        0x06 => '♠', // 2660 - ACKNOWLEDGE
        0x07 => '•', // 2022 - BELL
        0x08 => '◘', // 25D8 - BACKSPACE
        0x09 => '\t',// 25CB ○ - HORIZONTAL TABULATION
        0x0a => '\n',// 25D9 ◙ - LINE FEED
        0x0b => '♂', // 2642 - VERTICAL TABULATION
        0x0c => '♀', // 2640 - FORM FEED
        0x0d => ' ', // 266A ♪ - CARRIAGE RETURN
        0x0e => '♫', // 266B - SHIFT OUT
        0x0f => '☼', // 263C - SHIFT IN

        0x10 => '►', // 25BA - DATA LINK ESCAPE
        0x11 => '◄', // 25C4 - DEVICE CONTROL ONE
        0x12 => '↕', // 2195 - DEVICE CONTROL TWO
        0x13 => '‼', // 203C - DEVICE CONTROL THREE
        0x14 => '¶', // 00B6 - DEVICE CONTROL FOUR
        0x15 => '§', // 00A7 - NEGATIVE ACKNOWLEDGE
        0x16 => '▬', // 25AC - SYNCHRONOUS IDLE
        0x17 => '↨', // 21A8 - END OF TRANSMISSION BLOCK
        0x18 => '↑', // 2191 - CANCEL
        0x19 => '↓', // 2193 - END OF MEDIUM
        0x1a => '→', // 2192 - SUBSTITUTE
        0x1b => b as char, // 2190 - ESCAPE (2190 ←)
        0x1c => '∟', // 221F - FILE SEPARATOR
        0x1d => '↔', // 2194 - GROUP SEPARATOR
        0x1e => '▲', // 25B2 - RECORD SEPARATOR
        0x1f => '▼', // 25BC - UNIT SEPARATOR

        0x20 => ' ', // 0020 - SPACE
        0x21 => '!', // 0021 - EXCLAMATION MARK
        0x22 => '\'',// 0022 - QUOTATION MARK
        0x23 => '#', // 0023 - NUMBER SIGN
        0x24 => '$', // 0024 - DOLLAR SIGN
        0x25 => '%', // 0025 - PERCENT SIGN
        0x26 => '&', // 0026 - AMPERSAND
        0x27 => '\'',// 0027 - APOSTROPHE
        0x28 => '(', // 0028 - LEFT PARENTHESIS
        0x29 => ')', // 0029 - RIGHT PARENTHESIS
        0x2a => '*', // 002a - ASTERISK
        0x2b => '+', // 002b - PLUS SIGN
        0x2c => ',', // 002c - COMMA
        0x2d => '-', // 002d - HYPHEN-MINUS
        0x2e => '.', // 002e - FULL STOP
        0x2f => '/', // 002f - SOLIDUS

        0x30 => '0', // 0030 - DIGIT ZERO
        0x31 => '1', // 0031 - DIGIT ONE
        0x32 => '2', // 0032 - DIGIT TWO
        0x33 => '3', // 0033 - DIGIT THREE
        0x34 => '4', // 0034 - DIGIT FOUR
        0x35 => '5', // 0035 - DIGIT FIVE
        0x36 => '6', // 0036 - DIGIT SIX
        0x37 => '7', // 0037 - DIGIT SEVEN
        0x38 => '8', // 0038 - DIGIT EIGHT
        0x39 => '9', // 0039 - DIGIT NINE
        0x3a => ',', // 003a - COLON
        0x3b => ';', // 003b - SEMICOLON
        0x3c => '<', // 003c - LESS-THAN SIGN
        0x3d => '=', // 003d - EQUALS SIGN
        0x3e => '>', // 003e - GREATER-THAN SIGN
        0x3f => '?', // 003f - QUESTION MARK

        0x40 => '@', // 0040 - COMMERCIAL AT
        0x41 => 'A', // 0041 - LATIN CAPITAL LETTER A
        0x42 => 'B', // 0042 - LATIN CAPITAL LETTER B
        0x43 => 'C', // 0043 - LATIN CAPITAL LETTER C
        0x44 => 'D', // 0044 - LATIN CAPITAL LETTER D
        0x45 => 'E', // 0045 - LATIN CAPITAL LETTER E
        0x46 => 'F', // 0046 - LATIN CAPITAL LETTER F
        0x47 => 'G', // 0047 - LATIN CAPITAL LETTER G
        0x48 => 'H', // 0048 - LATIN CAPITAL LETTER H
        0x49 => 'I', // 0049 - LATIN CAPITAL LETTER I
        0x4a => 'J', // 004a - LATIN CAPITAL LETTER J
        0x4b => 'K', // 004b - LATIN CAPITAL LETTER K
        0x4c => 'L', // 004c - LATIN CAPITAL LETTER L
        0x4d => 'M', // 004d - LATIN CAPITAL LETTER M
        0x4e => 'N', // 004e - LATIN CAPITAL LETTER N
        0x4f => 'O', // 004f - LATIN CAPITAL LETTER O

        0x50 => 'P', // 0050 - LATIN CAPITAL LETTER P
        0x51 => 'Q', // 0051 - LATIN CAPITAL LETTER Q
        0x52 => 'R', // 0052 - LATIN CAPITAL LETTER R
        0x53 => 'S', // 0053 - LATIN CAPITAL LETTER S
        0x54 => 'T', // 0054 - LATIN CAPITAL LETTER T
        0x55 => 'U', // 0055 - LATIN CAPITAL LETTER U
        0x56 => 'V', // 0056 - LATIN CAPITAL LETTER V
        0x57 => 'W', // 0057 - LATIN CAPITAL LETTER W
        0x58 => 'X', // 0058 - LATIN CAPITAL LETTER X
        0x59 => 'Y', // 0059 - LATIN CAPITAL LETTER Y
        0x5a => 'Z', // 005a - LATIN CAPITAL LETTER Z
        0x5b => '[', // 005b - LEFT SQUARE BRACKET
        0x5c => '\\',// 005c - REVERSE SOLIDUS
        0x5d => ']', // 005d - RIGHT SQUARE BRACKET
        0x5e => '^', // 005e - CIRCUMFLEX ACCENT
        0x5f => '_', // 005f - LOW LINE

        0x60 => '`', // 0060 - GRAVE ACCENT
        0x61 => 'a', // 0061 - LATIN SMALL LETTER A
        0x62 => 'b', // 0062 - LATIN SMALL LETTER B
        0x63 => 'c', // 0063 - LATIN SMALL LETTER C
        0x64 => 'd', // 0064 - LATIN SMALL LETTER D
        0x65 => 'e', // 0065 - LATIN SMALL LETTER E
        0x66 => 'f', // 0066 - LATIN SMALL LETTER F
        0x67 => 'g', // 0067 - LATIN SMALL LETTER G
        0x68 => 'h', // 0068 - LATIN SMALL LETTER H
        0x69 => 'i', // 0069 - LATIN SMALL LETTER I
        0x6a => 'j', // 006a - LATIN SMALL LETTER J
        0x6b => 'k', // 006b - LATIN SMALL LETTER K
        0x6c => 'l', // 006c - LATIN SMALL LETTER L
        0x6d => 'm', // 006d - LATIN SMALL LETTER M
        0x6e => 'n', // 006e - LATIN SMALL LETTER N
        0x6f => 'o', // 006f - LATIN SMALL LETTER O

        0x70 => 'p', // 0070 - LATIN SMALL LETTER P
        0x71 => 'q', // 0071 - LATIN SMALL LETTER Q
        0x72 => 'r', // 0072 - LATIN SMALL LETTER R
        0x73 => 's', // 0073 - LATIN SMALL LETTER S
        0x74 => 't', // 0074 - LATIN SMALL LETTER T
        0x75 => 'u', // 0075 - LATIN SMALL LETTER U
        0x76 => 'v', // 0076 - LATIN SMALL LETTER V
        0x77 => 'w', // 0077 - LATIN SMALL LETTER W
        0x78 => 'x', // 0078 - LATIN SMALL LETTER X
        0x79 => 'y', // 0079 - LATIN SMALL LETTER Y
        0x7a => 'z', // 007a - LATIN SMALL LETTER Z
        0x7b => '{', // 007b - LEFT CURLY BRACKET
        0x7c => '-', // 007c - VERTICAL LINE
        0x7d => '}', // 007d - RIGHT CURLY BRACKET
        0x7e => '~', // 007e - TILDE
        0x7f => '⌂', // 2302 - DELETE

        0x80 => 'Ç', // 00c7 - LATIN CAPITAL LETTER C WITH CEDILLA
        0x81 => 'ü', // 00fc - LATIN SMALL LETTER U WITH DIAERESIS
        0x82 => 'é', // 00e9 - LATIN SMALL LETTER E WITH ACUTE
        0x83 => 'â', // 00e2 - LATIN SMALL LETTER A WITH CIRCUMFLEX
        0x84 => 'ä', // 00e4 - LATIN SMALL LETTER A WITH DIAERESIS
        0x85 => 'à', // 00e0 - LATIN SMALL LETTER A WITH GRAVE
        0x86 => 'å', // 00e5 - LATIN SMALL LETTER A WITH RING ABOVE
        0x87 => 'ç', // 00e7 - LATIN SMALL LETTER C WITH CEDILLA
        0x88 => 'ê', // 00ea - LATIN SMALL LETTER E WITH CIRCUMFLEX
        0x89 => 'ë', // 00eb - LATIN SMALL LETTER E WITH DIAERESIS
        0x8a => 'è', // 00e8 - LATIN SMALL LETTER E WITH GRAVE
        0x8b => 'ï', // 00ef - LATIN SMALL LETTER I WITH DIAERESIS
        0x8c => 'î', // 00ee - LATIN SMALL LETTER I WITH CIRCUMFLEX
        0x8d => 'ì', // 00ec - LATIN SMALL LETTER I WITH GRAVE
        0x8e => 'Ä', // 00c4 - LATIN CAPITAL LETTER A WITH DIAERESIS
        0x8f => 'Å', // 00c5 - LATIN CAPITAL LETTER A WITH RING ABOVE

        0x90 => 'É', // 00c9 - LATIN CAPITAL LETTER E WITH ACUTE
        0x91 => 'æ', // 00e6 - LATIN SMALL LIGATURE AE
        0x92 => 'Æ', // 00c6 - LATIN CAPITAL LIGATURE AE
        0x93 => 'ô', // 00f4 - LATIN SMALL LETTER O WITH CIRCUMFLEX
        0x94 => 'ö', // 00f6 - LATIN SMALL LETTER O WITH DIAERESIS
        0x95 => 'ò', // 00f2 - LATIN SMALL LETTER O WITH GRAVE
        0x96 => 'û', // 00fb - LATIN SMALL LETTER U WITH CIRCUMFLEX
        0x97 => 'ù', // 00f9 - LATIN SMALL LETTER U WITH GRAVE
        0x98 => 'ÿ', // 00ff - LATIN SMALL LETTER Y WITH DIAERESIS
        0x99 => 'Ö', // 00d6 - LATIN CAPITAL LETTER O WITH DIAERESIS
        0x9a => 'Ü', // 00dc - LATIN CAPITAL LETTER U WITH DIAERESIS
        0x9b => '¢', // 00a2 - CENT SIGN
        0x9c => '£', // 00a3 - POUND SIGN
        0x9d => '¥', // 00a5 - YEN SIGN
        0x9e => '₧', // 20a7 - PESETA SIGN
        0x9f => 'ƒ', // 0192 - LATIN SMALL LETTER F WITH HOOK

        0xa0 => 'á', // 00e1 - LATIN SMALL LETTER A WITH ACUTE
        0xa1 => 'í', // 00ed - LATIN SMALL LETTER I WITH ACUTE
        0xa2 => 'ó', // 00f3 - LATIN SMALL LETTER O WITH ACUTE
        0xa3 => 'ú', // 00fa - LATIN SMALL LETTER U WITH ACUTE
        0xa4 => 'ñ', // 00f1 - LATIN SMALL LETTER N WITH TILDE
        0xa5 => 'Ñ', // 00d1 - LATIN CAPITAL LETTER N WITH TILDE
        0xa6 => 'ª', // 00aa - FEMININE ORDINAL INDICATOR
        0xa7 => 'º', // 00ba - MASCULINE ORDINAL INDICATOR
        0xa8 => '¿', // 00bf - INVERTED QUESTION MARK
        0xa9 => '⌐', // 2310 - REVERSED NOT SIGN
        0xaa => '¬', // 00ac - NOT SIGN
        0xab => '½', // 00bd - VULGAR FRACTION ONE HALF
        0xac => '¼', // 00bc - VULGAR FRACTION ONE QUARTER
        0xad => '¡', // 00a1 - INVERTED EXCLAMATION MARK
        0xae => '«', // 00ab - LEFT-POINTING DOUBLE ANGLE QUOTATION MARK
        0xaf => '»', // 00bb - RIGHT-POINTING DOUBLE ANGLE QUOTATION MARK

        0xb0 => '░', // 2591 - LIGHT SHADE
        0xb1 => '▒', // 2592 - MEDIUM SHADE
        0xb2 => '▓', // 2593 - DARK SHADE
        0xb3 => '│', // 2502 - BOX DRAWINGS LIGHT VERTICAL
        0xb4 => '┤', // 2524 - BOX DRAWINGS LIGHT VERTICAL AND LEFT
        0xb5 => '╡', // 2561 - BOX DRAWINGS VERTICAL SINGLE AND LEFT DOUBLE
        0xb6 => '╢', // 2562 - BOX DRAWINGS VERTICAL DOUBLE AND LEFT SINGLE
        0xb7 => '╖', // 2556 - BOX DRAWINGS DOWN DOUBLE AND LEFT SINGLE
        0xb8 => '╕', // 2555 - BOX DRAWINGS DOWN SINGLE AND LEFT DOUBLE
        0xb9 => '╣', // 2563 - BOX DRAWINGS DOUBLE VERTICAL AND LEFT
        0xba => '║', // 2551 - BOX DRAWINGS DOUBLE VERTICAL
        0xbb => '╗', // 2557 - BOX DRAWINGS DOUBLE DOWN AND LEFT
        0xbc => '╝', // 255d - BOX DRAWINGS DOUBLE UP AND LEFT
        0xbd => '╜', // 255c - BOX DRAWINGS UP DOUBLE AND LEFT SINGLE
        0xbe => '╛', // 255b - BOX DRAWINGS UP SINGLE AND LEFT DOUBLE
        0xbf => '┐', // 2510 - BOX DRAWINGS LIGHT DOWN AND LEFT

        0xc0 => '└', // 2514 - BOX DRAWINGS LIGHT UP AND RIGHT
        0xc1 => '┴', // 2534 - BOX DRAWINGS LIGHT UP AND HORIZONTAL
        0xc2 => '┬', // 252c - BOX DRAWINGS LIGHT DOWN AND HORIZONTAL
        0xc3 => '├', // 251c - BOX DRAWINGS LIGHT VERTICAL AND RIGHT
        0xc4 => '─', // 2500 - BOX DRAWINGS LIGHT HORIZONTAL
        0xc5 => '┼', // 253c - BOX DRAWINGS LIGHT VERTICAL AND HORIZONTAL
        0xc6 => '╞', // 255e - BOX DRAWINGS VERTICAL SINGLE AND RIGHT DOUBLE
        0xc7 => '╟', // 255f - BOX DRAWINGS VERTICAL DOUBLE AND RIGHT SINGLE
        0xc8 => '╚', // 255a - BOX DRAWINGS DOUBLE UP AND RIGHT
        0xc9 => '╔', // 2554 - BOX DRAWINGS DOUBLE DOWN AND RIGHT
        0xca => '╩', // 2569 - BOX DRAWINGS DOUBLE UP AND HORIZONTAL
        0xcb => '╦', // 2566 - BOX DRAWINGS DOUBLE DOWN AND HORIZONTAL
        0xcc => '╠', // 2560 - BOX DRAWINGS DOUBLE VERTICAL AND RIGHT
        0xcd => '═', // 2550 - BOX DRAWINGS DOUBLE HORIZONTAL
        0xce => '╬', // 256c - BOX DRAWINGS DOUBLE VERTICAL AND HORIZONTAL
        0xcf => '╶', // 2567 - BOX DRAWINGS UP SINGLE AND HORIZONTAL DOUBLE

        0xd0 => '╸', // 2568 - BOX DRAWINGS UP DOUBLE AND HORIZONTAL SINGLE
        0xd1 => '╤', // 2564 - BOX DRAWINGS DOWN SINGLE AND HORIZONTAL DOUBLE
        0xd2 => '╥', // 2565 - BOX DRAWINGS DOWN DOUBLE AND HORIZONTAL SINGLE
        0xd3 => '╙', // 2559 - BOX DRAWINGS UP DOUBLE AND RIGHT SINGLE
        0xd4 => '╘', // 2558 - BOX DRAWINGS UP SINGLE AND RIGHT DOUBLE
        0xd5 => '╒', // 2552 - BOX DRAWINGS DOWN SINGLE AND RIGHT DOUBLE
        0xd6 => '╓', // 2553 - BOX DRAWINGS DOWN DOUBLE AND RIGHT SINGLE
        0xd7 => '╫', // 256b - BOX DRAWINGS VERTICAL DOUBLE AND HORIZONTAL SINGLE
        0xd8 => '╪', // 256a - BOX DRAWINGS VERTICAL SINGLE AND HORIZONTAL DOUBLE
        0xd9 => '┘', // 2518 - BOX DRAWINGS LIGHT UP AND LEFT
        0xda => '┌', // 250c - BOX DRAWINGS LIGHT DOWN AND RIGHT
        0xdb => '█', // 2588 - FULL BLOCK
        0xdc => '▄', // 2584 - LOWER HALF BLOCK
        0xdd => '▌', // 258c - LEFT HALF BLOCK
        0xde => '▐', // 2590 - RIGHT HALF BLOCK
        0xdf => '▀', // 2580 - UPPER HALF BLOCK

        0xe0 => 'ʱ', // 03b1 - GREEK SMALL LETTER ALPHA
        0xe1 => 'ß', // 00df - LATIN SMALL LETTER SHARP S
        0xe2 => 'γ', // 0393 - GREEK CAPITAL LETTER GAMMA
        0xe3 => 'π', // 03c0 - GREEK SMALL LETTER PI
        0xe4 => 'Σ', // 03a3 - GREEK CAPITAL LETTER SIGMA
        0xe5 => 'σ', // 03c3 - GREEK SMALL LETTER SIGMA
        0xe6 => 'µ', // 00b5 - MICRO SIGN
        0xe7 => 'τ', // 03c4 - GREEK SMALL LETTER TAU
        0xe8 => 'Φ', // 03a6 - GREEK CAPITAL LETTER PHI
        0xe9 => 'Θ', // 0398 - GREEK CAPITAL LETTER THETA
        0xea => 'Ω', // 03a9 - GREEK CAPITAL LETTER OMEGA
        0xeb => 'δ', // 03b4 - GREEK SMALL LETTER DELTA
        0xec => '∞', // 221e - INFINITY
        0xed => 'φ', // 03c6 - GREEK SMALL LETTER PHI
        0xee => 'ε', // 03b5 - GREEK SMALL LETTER EPSILON
        0xef => '∩', // 2229 - INTERSECTION

        0xf0 => '≡', // 2261 - IDENTICAL TO
        0xf1 => '±', // 00b1 - PLUS-MINUS SIGN
        0xf2 => '≥', // 2265 - GREATER-THAN OR EQUAL TO
        0xf3 => '≤', // 2264 - LESS-THAN OR EQUAL TO
        0xf4 => '⌠', // 2320 - TOP HALF INTEGRAL
        0xf5 => '⌡', // 2321 - BOTTOM HALF INTEGRAL
        0xf6 => '÷', // 00f7 - DIVISION SIGN
        0xf7 => '≈', // 2248 - ALMOST EQUAL TO
        0xf8 => '°', // 00b0 - DEGREE SIGN
        0xf9 => '∙', // 2219 - BULLET OPERATOR
        0xfa => '·', // 00b7 - MIDDLE DOT
        0xfb => '√', // 221a - SQUARE ROOT
        0xfc => 'ⁿ', // 207f - SUPERSCRIPT LATIN SMALL LETTER N
        0xfd => '²', // 00b2 - SUPERSCRIPT TWO
        0xfe => '■', // 25a0 - BLACK SQUARE
        0xff => ' ', // 00a0 - NO-BREAK SPACE
    }
}
