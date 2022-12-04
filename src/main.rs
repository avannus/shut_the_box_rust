extern crate rustop;

type Uns = u8;
type Float = f64;
type Tiles = Vec<Uns>;

use std::collections::HashMap;

#[derive(Debug)]
struct Trunk {
    game_meta: GameMeta,

    // todo prob reference GameState owned by the HashMap later
    start_tiles: Tiles,

    game_db: HashMap<Tiles, Float>, // contains all possible child game states
}

#[derive(Debug)]
struct GameMeta {
    die_max: Uns, // maximum die value

    trphm: HashMap<Uns, Vec<Tiles>>, // tile removal possibilities hash map
    roll_probs_single: HashMap<Uns, Float>, // probabilities of key if a single die is rolled
    roll_probs_multi: HashMap<Uns, Float>, // probabilities of key if multiple dice are rolled
}

/// TODOS
/// For now, all data is being cloned for everything, which is not ideal
/// General optimizations
///     Is there a smarter way to get the game metadata, esp when sorted?
/// General Improvements
///     Allow to roll any number of dice, not just multi and single
/// Remove #[derive(Debug)] from structs
fn main() {
    // TODO make these command line args
    let die_vals = vec![1, 2, 3, 4, 5, 6];
    let die_max = get_max(&die_vals);
    let die_cnt = 2;
    let start_tiles = get_srt(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let max_remove = 0;

    // todo probably can optimize with this sorted
    // todo eventually make this where the num dice rolled is totally dynamic
    let all_multi_rolls = get_srt(&get_roll_counts(&die_vals, die_cnt, 0));
    let roll_probs_multi = get_roll_probabilities(&all_multi_rolls);

    let all_single_rolls = get_srt(&get_roll_counts(&die_vals, 1, 0));
    let roll_probs_single = get_roll_probabilities(&all_single_rolls);

    let roll_possib = get_srt_dedup_keys(&roll_probs_multi, &roll_probs_single);

    let trphm = get_tile_removal_possibilities(&start_tiles, &roll_possib, &max_remove);
    // println!("roll_counts: {:?}", all_multi_rolls);
    // println!("possible_rolls: {:?}", roll_possib);
    // println!("roll_probabilities: {:?}", roll_probs_multi);
    // println!("roll_probabilities_single: {:?}", roll_probs_single);
    // println!("trphm: {:?}", trphm);

    let mut game_db: HashMap<Tiles, Float> = HashMap::new();
    let game_meta = GameMeta {
        die_max,
        trphm,
        roll_probs_single,
        roll_probs_multi,
    };
    // fill out game_db
    r_solve(start_tiles.clone(), &game_meta, &mut game_db);
    let trunk = Trunk {
        game_meta,
        start_tiles,
        game_db,
    };
    println!("\n\nGame Data:\n{}", get_readable_string(&trunk));
}

fn get_readable_string(trunk: &Trunk) -> String {
    let mut s = String::new();
    s.push_str("game_meta:\n ");
    s.push_str(&format!("{:?}", trunk.game_meta));
    s.push_str("\nstart_tiles:\n ");
    s.push_str(&format!("{:?}", trunk.start_tiles));
    s.push_str("\ngame_db entry for start tiles:\n ");
    s.push_str(&format!(
        "{:?}",
        trunk.game_db.get(&trunk.start_tiles).unwrap()
    ));
    s.push_str("\n");
    s
}

fn r_solve(
    tiles: Tiles,
    game_meta: &GameMeta,
    game_db: &mut HashMap<Tiles, Float>,
) -> Float {
    let tiles = tiles.clone();
    let existing_game = game_db.get(&tiles);
    match existing_game {
        Some(existing_game) => {
            return existing_game.clone();
        }
        None => {
            if tiles.len() == 0 {
                let win = 1.;
                game_db.insert(tiles, win);
                return win;
            }
            /*
            If the game doesn't exist in the hm, we need to create it
            To create a game, we need:
                win_chance
                win_chance_single
                win_chance_multi

                next_states
                child_count
                optimal_next_states_single
                optimal_next_states_multi
            Probably solve in this order:
                next_states
                    solve next_stats
                child_count
                optimal_next_states_single
                optimal_next_states_multi

                win_single
                win_multi
                win_chance
            */
            let all_next_legal_states_hm = get_next_legal_states_all(&tiles, &game_meta.trphm);
            let solved_next_legal_states_hm =
                get_all_stats_from_hm(&all_next_legal_states_hm, game_meta, game_db);

            let mut best_states_single_hm = HashMap::new();
            let mut best_states_multi_hm = HashMap::new();
            let mut win_chance_single = 0.;
            let mut win_chance_multi = 0.;
            for (roll, state_stat) in solved_next_legal_states_hm {
                let mut curr_best_chance = 0.;
                let mut curr_best_state: Tiles = vec![];
                let mut curr_best_win_chance = 0.;
                for (state, win_chance) in state_stat {
                    if win_chance > curr_best_chance {
                        curr_best_chance = win_chance;
                        curr_best_state = state;
                        curr_best_win_chance = win_chance;
                    }
                }
                if get_single_legality(&tiles, &game_meta.die_max) {
                    let single_chance = game_meta.roll_probs_single.get(&roll);
                    match single_chance {
                        Some(single_chance) => {
                            best_states_single_hm.insert(roll, curr_best_state.clone());
                            win_chance_single += curr_best_win_chance * single_chance;
                        }
                        None => {}
                    }
                }
                let multi_chance = game_meta.roll_probs_multi.get(&roll);
                match multi_chance {
                    Some(multi_chance) => {
                        best_states_multi_hm.insert(roll, curr_best_state.clone());
                        win_chance_multi += curr_best_win_chance * multi_chance;
                    }
                    None => {}
                }
            }
            let win_chance = win_chance_single.max(win_chance_multi);
            game_db.insert(tiles, win_chance);
            win_chance
        }
    }
}

fn get_next_legal_states_roll(tiles: &Tiles, trps: &Vec<Tiles>) -> Vec<Tiles> {
    let mut legal_states = Vec::new();
    for trp in trps {
        let new_tiles = get_removed_tiles(tiles, trp);
        match new_tiles {
            Some(new_tiles) => {
                legal_states.push(new_tiles);
            }
            None => {}
        }
    }
    legal_states
}

fn get_next_legal_states_all(
    tiles: &Tiles,
    trphm: &HashMap<Uns, Vec<Tiles>>,
) -> HashMap<Uns, Vec<Tiles>> {
    let mut hm = HashMap::new();
    if tiles.len() == 0 {
        return hm;
    }
    for (roll, trps) in trphm {
        let legal_states = get_next_legal_states_roll(tiles, trps);
        if legal_states.len() > 0 {
            hm.insert(*roll, legal_states);
        }
    }
    hm
}

fn get_all_stats_from_states(
    states: &Vec<Tiles>,
    game_meta: &GameMeta,
    game_db: &mut HashMap<Tiles, Float>,
) -> Vec<(Tiles, Float)> {
    let mut res = Vec::new();
    for state in states {
        let state = state.clone();
        let stats = r_solve(state.clone(), game_meta, game_db);
        res.push((state, stats));
    }
    res
}

fn get_all_stats_from_hm(
    state_hm: &HashMap<Uns, Vec<Tiles>>,
    game_meta: &GameMeta,
    game_db: &mut HashMap<Tiles, Float>,
) -> HashMap<Uns, Vec<(Tiles, Float)>> {
    let mut hm = HashMap::new();
    for (roll, game_states) in state_hm {
        let games = get_all_stats_from_states(&game_states.clone(), game_meta, game_db);
        hm.insert(*roll, games);
    }
    hm
}

fn get_single_legality(tiles: &Tiles, max_die: &Uns) -> bool {
    tiles.len() > 0 && tiles.iter().max().unwrap() <= max_die
}

fn get_max(vals: &[Uns]) -> Uns {
    vals.iter().max().unwrap().clone()
}

fn get_removed_tiles(tiles: &Tiles, trp: &Tiles) -> Option<Tiles> {
    let mut new_tiles = tiles.clone();
    for &tile in trp {
        if new_tiles.contains(&tile) {
            new_tiles.remove(new_tiles.iter().position(|&x| x == tile).unwrap());
        } else {
            return None;
        }
    }
    Some(new_tiles)
}

///// SETUP FUNCTIONS /////

fn get_roll_counts(values: &Vec<Uns>, count: Uns, sum: Uns) -> Vec<Uns> {
    let mut counts = Vec::new();
    if count == 0 {
        counts.push(sum);
    } else {
        for value in values {
            counts.append(&mut get_roll_counts(&values, count - 1, sum + value));
        }
    }
    counts
}

/**
 * returns an sorted vector
 */
fn get_srt<T: Copy + Ord>(a: &[T]) -> Vec<T> {
    let mut b = a.to_vec();
    b.sort_unstable();
    b
}

fn get_srt_dedup_keys<T, U>(hm1: &HashMap<Uns, T>, hm2: &HashMap<Uns, U>) -> Vec<Uns> {
    let mut x = hm1.keys().map(|&x| x).collect::<Vec<Uns>>();
    x.append(&mut hm2.keys().map(|&x| x).collect::<Vec<Uns>>());
    x = get_srt(&x);
    x.dedup();
    x
}

fn get_roll_probabilities(rolls: &Vec<Uns>) -> HashMap<Uns, Float> {
    let mut roll_counts: HashMap<Uns, usize> = HashMap::new();
    let mut roll_probabilities: HashMap<Uns, Float> = HashMap::new();
    let mut total_rolls = 0;
    for roll in rolls {
        let count = roll_counts.entry(*roll).or_insert(0);
        *count += 1;
        total_rolls += 1;
    }
    for (roll, count) in roll_counts {
        roll_probabilities.insert(roll, count as Float / total_rolls as Float);
    }
    roll_probabilities
}

fn get_tile_removal_possibilities(
    tiles: &Tiles,
    possible_rolls: &Vec<Uns>,
    removal_max: &Uns,
) -> HashMap<Uns, Vec<Tiles>> {
    let mut trp: HashMap<Uns, Vec<Tiles>> = HashMap::new();
    for roll in possible_rolls {
        let removals: Vec<Tiles> = r_tile_removal(tiles, roll, &removal_max);
        trp.insert(*roll, removals);
    }
    trp
}

fn r_tile_removal(tiles: &[Uns], targ: &Uns, removal_max: &Uns) -> Vec<Tiles> {
    let mut removals: Vec<Tiles> = Vec::new();
    if targ == &0 {
        removals.push(Vec::new());
        return removals;
    }
    if removal_max == &1 {
        for tile in tiles {
            if tile == targ {
                removals.push(vec![*tile]);
            }
        }
    } else {
        for tile in tiles {
            if tile <= targ {
                let start = tiles.iter().position(|&x| x == *tile).unwrap();
                let new_tiles = &tiles[start + 1..];
                let new_removal_max = if removal_max > &1 { removal_max - 1 } else { 0 };
                let new_removals = r_tile_removal(new_tiles, &(targ - tile), &new_removal_max);
                for mut removal in new_removals {
                    removal.push(*tile);
                    removals.push(removal);
                }
            }
        }
    }
    removals
}
