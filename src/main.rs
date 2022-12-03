extern crate rustop;

use std::{collections::HashMap, hash::Hash};

// todo implement
// struct Game {
//     game_stats: GameStats,
//     game_state: GameState,
//     game_data: GameData,
// }

#[derive(Debug, Clone)]
struct GameStats <'g_state> {
    win_chance: f64,        // chance of winning
    win_chance_single: f64, // chance of winning with a single die
    win_chance_all: f64, // chance of winning with all dice

    child_count: usize,                                  // number of children
    next_states: HashMap<u8, Vec<&'g_state GameState<'g_state>>>,          // all possible next states keyed by roll
    optimal_next_states_single: HashMap<u8, &'g_state GameState<'g_state>>, // optimal next states with one die
    optimal_next_states_double: HashMap<u8, &'g_state GameState<'g_state>>, // optimal next states with two dice
}

#[derive(Eq, PartialEq, Hash, Clone, Debug)] // todo remove debug
struct GameState <'g_rules> {
    die_vals: &'g_rules Vec<u8>, // all possible die values
    die_cnt: &'g_rules u8,       // number of dice
    tiles: &'g_rules Vec<u8>,    // remaining tiles
    max_remove: &'g_rules u8,    // max number of tiles to remove, 0 for unlimited
}

// todo implement
// struct GameData {
//     trphm: HashMap<u8, Vec<Vec<u8>>>,
//     game_db: HashMap<GameState, GameStats>,
//     roll_probs_single: HashMap<u8, f64>,
//     roll_probs_double: HashMap<u8, f64>,
// }



fn get_best_game_state(
    game: Vec<(GameState, GameStats)>,
) -> &'static GameState<'static> {
    let mut best_game_state = &game[0].0;
    let mut best_game_stats = &game[0].1;
    for (game_state, game_stats) in game {
        if game_stats.win_chance > best_game_stats.win_chance {
            best_game_state = &game_state;
            best_game_stats = &game_stats;
        }
    }
    best_game_state
}
fn main() {
    let die_vals: Vec<u8> = [1, 2, 3, 4, 5, 6].to_vec();
    let die_cnt: u8 = 2;
    let tiles: Vec<u8> = get_srt(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let max_remove: u8 = 0;

    // todo probably can optimize with this sorted
    let all_rolls = get_srt(&get_roll_counts(&die_vals, die_cnt, 0));
    let roll_probs_double = get_roll_probabilities(&all_rolls);
    let roll_probs_single = get_roll_probabilities_single(&die_vals);
    let roll_possib = get_srt_dedup_keys(&roll_probs_double, &roll_probs_single);

    let trphm = get_tile_removal_possibilities(&tiles, &roll_possib, &max_remove);
    println!("roll_counts: {:?}", all_rolls);
    println!("possible_rolls: {:?}", roll_possib);
    println!("roll_probabilities: {:?}", roll_probs_double);
    println!("roll_probabilities_single: {:?}", roll_probs_single);
    println!("trphm: {:?}", trphm);

    let mut game_db: HashMap<GameState, GameStats> = HashMap::new();

    let game_state = GameState {
        die_vals,
        die_cnt,
        tiles,
        max_remove,
    };

    let x = r_solve(
        &game_state,
        &roll_probs_double,
        &roll_probs_single,
        &trphm,
        &mut game_db,
    );
    println!("{:?}", x);
}

fn get_srt<T: Clone + Ord>(a: &[T]) -> Vec<T> {
    let mut b = a.to_vec();
    b.sort_unstable();
    b
}

fn get_srt_dedup_keys(hm1: &HashMap<u8, f64>, hm2: &HashMap<u8, f64>) -> Vec<u8> {
    let mut x = hm1.keys().map(|&x| x).collect::<Vec<u8>>();
    x.append(&mut hm2.keys().map(|&x| x).collect::<Vec<u8>>());
    x = get_srt(&x);
    x.dedup();
    x
}

fn get_roll_probabilities_single(die_vals: &Vec<u8>) -> HashMap<u8, f64> {
    let mut roll_probs = HashMap::new();
    for &die_val in die_vals {
        roll_probs.insert(die_val, 1.0 / die_vals.len() as f64);
    }
    roll_probs
}

fn get_single_legality(game_state: &GameState) -> bool {
    let tiles = &game_state.tiles;
    let die_vals = &game_state.die_vals;
    tiles.len() > 0 && tiles.iter().max().unwrap() <= die_vals.iter().max().unwrap()
}

fn r_solve(
    game_state: &GameState,
    roll_probs_double: &HashMap<u8, f64>,
    roll_probs_single: &HashMap<u8, f64>,
    trphm: &HashMap<u8, Vec<Vec<u8>>>,
    game_db: &mut HashMap<GameState, GameStats>,
) -> &'static GameStats {
    let existing_game = game_db.get(game_state);
    match existing_game {
        Some(existing_game) => {
            return existing_game;
        }
        None => { // todo review this, weird copying for no real good reason besides simplicity to code
            if game_state.tiles.len() == 0 {
                let x = GameStats {
                    win_chance: 1.0,
                    win_chance_single: 1.0,
                    win_chance_all: 1.0,
                    child_count: 0,
                    next_states: HashMap::new(),
                    optimal_next_states_single: HashMap::new(),
                    optimal_next_states_double: HashMap::new(),
                };
                game_db.insert(*game_state, x);
                return &x;
            }
            let next_valid_states_hm = get_next_legal_state_hm(game_state, trphm);

            // single
            let mut next_valid_states_single = HashMap::new();
            // TODO rework to a single loop where next_valid_states_hm.get(roll) is called once per roll
            for (roll, _) in roll_probs_single {
                next_valid_states_single.insert(
                    *roll,
                    next_valid_states_hm.get(roll).unwrap(),
                );
            }
            let mut next_best_states_single = HashMap::new();
            for (roll, next_valid_states_single) in next_valid_states_single {
                let mut curr_roll_games = Vec::new();
                for next_valid_state_single in next_valid_states_single {
                    let next_state_single = r_solve(
                        next_valid_state_single,
                        roll_probs_double,
                        roll_probs_single,
                        trphm,
                        game_db,
                    );
                    curr_roll_games.push((*next_valid_state_single, *next_state_single));
                }
                let next_best_state_single = get_best_game_state(curr_roll_games);
                next_best_states_single.insert(roll, next_best_state_single);
            }
            let win_chance_single = get_win_chance_single(
                game_state,
                roll_probs_single,
                game_db,
                &next_best_states_single,
            );

            // all dice
            let mut next_valid_states_double= HashMap::new();
            for (roll, _) in roll_probs_double {
                next_valid_states_double.insert(
                    *roll,
                    next_valid_states_hm.get(roll).unwrap(),
                );
            }
            let mut next_best_states_double = HashMap::new();
            for (roll, next_valid_states_double) in next_valid_states_double {
                let mut curr_roll_games = Vec::new();
                for next_valid_state_double in next_valid_states_double {
                    let next_state_double = r_solve(
                        next_valid_state_double,
                        roll_probs_double,
                        roll_probs_single,
                        trphm,
                        game_db,
                    );
                    curr_roll_games.push((*next_valid_state_double, *next_state_double));
                }
                let next_best_state_double = get_best_game_state(curr_roll_games);
                next_best_states_double.insert(roll, next_best_state_double);
            }
            let win_chance_double = get_win_chance_double(
                game_state,
                roll_probs_double,
                game_db,
                &next_best_states_double,
            );

            let win_chance = win_chance_single.max(win_chance_double);
            let child_count = next_valid_states_hm.len();
            let game_stats = GameStats {
                win_chance,
                win_chance_single,
                win_chance_all: win_chance_double,
                child_count,
                next_states: next_valid_states_hm,
                optimal_next_states_single: next_best_states_single,
                optimal_next_states_double: next_best_states_double,
            };
            game_db.insert(game_state.clone(), game_stats);
            &game_stats
        }
    }
}

fn get_next_legal_state_hm(
    game_state: &GameState,
    trphm: &HashMap<u8, Vec<Vec<u8>>>,
) -> HashMap<u8, Vec<GameState>> {
    let mut hm = HashMap::new();
    let tiles = &game_state.tiles;
    if tiles.len() == 0 {
        return hm;
    }
    for (roll, trps) in trphm {
        let mut legal_states = Vec::new();
        for trp in trps {
            let new_tiles = get_removed_tiles(tiles, trp);
            match new_tiles {
                Some(new_tiles) => {
                    let mut valid_state = game_state.clone(); // todo why can this not be mut
                    valid_state.tiles = new_tiles;
                    legal_states.push(valid_state);
                }
                None => {}
            }
        }
        if legal_states.len() > 0 {
            hm.insert(*roll, legal_states);
        }
    }
    hm
}

fn get_win_chance_double(
    game_state: &GameState,
    roll_probs_double: &HashMap<u8, f64>,
    game_db: &HashMap<GameState, GameStats>,
    next_best_states_double: &HashMap<u8, GameState>,
) -> f64 {
    let tiles = &game_state.tiles;
    if tiles.len() == 0 {
        return 1.; // todo check if this is correct
    }
    let mut prob = 0.;
    for (roll, roll_prob) in roll_probs_double {
        let best_state = next_best_states_double.get(roll).unwrap();
        prob += roll_prob * game_db.get(best_state).unwrap().win_chance;
    }
    prob
}

fn get_win_chance_single(
    game_state: &GameState,
    roll_probs_single: &HashMap<u8, f64>,
    game_db: &HashMap<GameState, GameStats>,
    next_best_states_single: &HashMap<u8, GameState>,
) -> f64 {
    let tiles = &game_state.tiles;
    if tiles.len() == 0 {
        return 1.; // todo check if this is correct
    }
    if !get_single_legality(game_state) {
        return 0.;
    }
    let mut prob = 0.;
    for (roll, roll_prob) in roll_probs_single {
        let best_state = next_best_states_single.get(roll).unwrap();
        prob += roll_prob * game_db.get(best_state).unwrap().win_chance;
    }
    prob
}

fn get_removed_tiles(tiles: &Vec<u8>, trp: &Vec<u8>) -> Option<Vec<u8>> {
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

fn get_tile_removal_possibilities(
    tiles: &Vec<u8>,
    possible_rolls: &Vec<u8>,
    removal_max: &u8,
) -> HashMap<u8, Vec<Vec<u8>>> {
    let mut trp: HashMap<u8, Vec<Vec<u8>>> = HashMap::new();
    for roll in possible_rolls {
        let removals: Vec<Vec<u8>> = r_tile_removal(tiles, roll, &removal_max);
        trp.insert(*roll, removals);
    }
    trp
}

fn r_tile_removal(tiles: &[u8], targ: &u8, removal_max: &u8) -> Vec<Vec<u8>> {
    let mut removals: Vec<Vec<u8>> = Vec::new();
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

fn get_roll_probabilities(rolls: &Vec<u8>) -> HashMap<u8, f64> {
    let mut roll_counts: HashMap<u8, u32> = HashMap::new();
    let mut roll_probabilities: HashMap<u8, f64> = HashMap::new();
    let mut total_rolls: u32 = 0;
    for roll in rolls {
        let count = roll_counts.entry(*roll).or_insert(0);
        *count += 1;
        total_rolls += 1;
    }
    for (roll, count) in roll_counts {
        roll_probabilities.insert(roll, count as f64 / total_rolls as f64);
    }
    roll_probabilities
}

fn get_roll_counts(values: &Vec<u8>, count: u8, sum: u8) -> Vec<u8> {
    let mut counts: Vec<u8> = Vec::new();
    if count == 0 {
        counts.push(sum);
    } else {
        for value in values {
            counts.append(&mut get_roll_counts(&values, count - 1, sum + value));
        }
    }
    counts
}
