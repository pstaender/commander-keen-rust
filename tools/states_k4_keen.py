"""Keen 4 states and functions of the shared CK_KEEN / CK_KEEN2 code
(Keen himself, the world map Keen, flags)."""

RENAME = {
    # moon easter egg (KeenPauseThink after standing on the moon tile)
    0x098a: 'S_KEENMOON1',
    0x09a8: 'S_KEENMOON2',
    0x09c6: 'S_KEENMOON3',
    # dying in the underwater level (KillKeen, map 17)
    0x0c3c: 'S_KEENSWIMDIE1',
    0x0c5a: 'S_KEENSWIMDIE2',
    # world map: riding the giant foot, swimming
    0x1382: 'S_KEENONFOOT1',
    0x13a0: 'S_KEENONFOOT2',
    0x13be: 'S_WORLDSWIM',
}

FUNCS = {
    0x1bffc: 'keen::keen_drop_down_think',
    0x1c00b: 'keen::keen_hold_think',
    0x1c304: 'keen::keen_contact',
    0x1c468: 'keen::keen_pos_contact',
    0x1c87f: 'keen::keen_pole_react',
    0x1cf69: 'ep::keen_slide_react',
    0x1d758: 'keen2::t_foot_fly',
    0x1d7d3: 'keen2::t_world_swim',
    0x1dbb4: 'keen2::t_flag_land',
}
