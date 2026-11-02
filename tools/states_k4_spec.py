"""Keen 4 CK4_SPEC states (swimming Keen, his air bubbles)."""

RENAME = {
    0x18dc: 'S_KEENSWIMSLOW1',
    0x18fa: 'S_KEENSWIMSLOW2',
    0x1918: 'S_KEENSWIM1',
    0x1936: 'S_KEENSWIM2',
    0x1954: 'S_KBUBBLE1',
    0x1972: 'S_KBUBBLE2',
    0x1990: 'S_KBUBBLE3',
    0x19ae: 'S_KBUBBLE4',
}

FUNCS = {
    0x1ec2f: 'spec::t_keen_swim_slow',
    0x1edc5: 'spec::t_keen_swim',
    0x1ee77: 'spec::c_keen_swim',
    0x1ef40: 'spec::r_keen_swim',
}
