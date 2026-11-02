"""Keen 5 states and functions of the shared CK_KEEN / CK_KEEN2 code
(Keen himself, the world map Keen and elevators, flags, shots, doors)."""

RENAME = {
    # standing on a platform: Keen faces the player
    0x08a6: 'S_KEENONPLATFORM',
    # waiting while the security door opens (CheckEnterHouse)
    0x0cde: 'S_KEENOPENCARDDOOR',
    # invisible after entering a teleporter / the exit door
    0x0db0: 'S_KEENENTER6',
    # world map: riding an elevator (invisible)
    0x137a: 'S_WORLDELEVATE',
    # the opening security door
    0x1610: 'S_CARDDOOR',
}

FUNCS = {
    0x1ae1b: 'keen::keen_stand_think',
    0x1af5d: 'keen::keen_pause_think',
    0x1baf1: 'keen::keen_pole_think',
    0x1bcef: 'keen::keen_drop_down_think',
    0x1bcfe: 'keen::keen_hold_think',
    0x1bf68: 'keen::t_pull_up1',
    0x1bf87: 'keen::t_pull_up2',
    0x1bfb2: 'keen::t_pull_up3',
    0x1bfbd: 'keen::t_pull_up_done',
    0x1c087: 'keen::keen_contact',
    0x1c16f: 'keen::keen_pos_contact',
    0x1c58f: 'keen::keen_pole_react',
    0x1cafb: 'keen::keen_pogo_react',
    0x1cc35: 'ep::keen_slide_react',
    0x1d719: 'keen2::t_elevate',
    0x1de52: 'keen2::r_shot',
    0x1dfa5: 'keen2::door_open',
    0x1e023: 'keen2::t_card_door',
}
