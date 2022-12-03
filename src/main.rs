extern crate rustop;

type Uns = u8;
type Float = f64;

use std::collections::HashMap;

#[derive(Debug)]
struct Trunk {
    game_rules: GameRules,
    game_meta: GameMeta,

    // todo prob reference Vec<Uns> owned by the HashMap later
    start_tiles: Vec<Uns>,

    game_db: HashMap<Vec<Uns>, GameStats>, // contains all possible child game states
}

#[derive(Debug)]
struct GameRules {
    die_vals: Vec<Uns>, // all possible die values
    die_cnt: Uns,       // number of dice
    max_remove: Uns,    // max number of tiles to remove, 0 for unlimited
}

#[derive(Debug)]
struct GameMeta {
    trphm: HashMap<Uns, Vec<Vec<Uns>>>, // tile removal possibilities hash map
    roll_probs_single: HashMap<Uns, Float>, // probability of winning if a single die is rolled
    roll_probs_multi: HashMap<Uns, Float>, // probability of winning if multiple dice are rolled
}

#[derive(Debug)]
struct GameStats {
    win_chance: Float,        // chance of winning
    win_chance_single: Float, // chance of winning with a single die
    win_chance_multi: Float,  // chance of winning with all dice

    child_count: usize,                                 // number of children
    next_states: HashMap<Uns, Vec<Vec<Uns>>>,           // all possible next states keyed by roll
    optimal_next_states_single: HashMap<Uns, Vec<Uns>>, // optimal next states with one die
    optimal_next_states_multi: HashMap<Uns, Vec<Uns>>,  // optimal next states with two dice
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
    let die_cnt = 2;
    let tiles = get_srt(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let max_remove = 0;

    // todo probably can optimize with this sorted
    // todo eventually make this where the num dice rolled is totally dynamic
    let all_multi_rolls = get_srt(&get_roll_counts(&die_vals, die_cnt, 0));
    let roll_probs_multi = get_roll_probabilities(&all_multi_rolls);

    let all_single_rolls = get_srt(&get_roll_counts(&die_vals, 1, 0));
    let roll_probs_single = get_roll_probabilities(&all_single_rolls);

    let roll_possib = get_srt_dedup_keys(&roll_probs_multi, &roll_probs_single);

    let trphm = get_tile_removal_possibilities(&tiles, &roll_possib, &max_remove);
    println!("roll_counts: {:?}", all_multi_rolls);
    println!("possible_rolls: {:?}", roll_possib);
    println!("roll_probabilities: {:?}", roll_probs_multi);
    println!("roll_probabilities_single: {:?}", roll_probs_single);
    println!("trphm: {:?}", trphm);

    /*
    Trunk needs:
    game_rules
        die_vals
        die_cnt
        max_remove
    game_meta
        trphm
        roll_probs_single
        roll_probs_multi
    start_tiles
    game_db
    */
    let mut game_db: HashMap<Vec<Uns>, GameStats> = HashMap::new();
    let game_rules = GameRules {
        die_vals,
        die_cnt,
        max_remove,
    };
    let game_meta = GameMeta {
        trphm,
        roll_probs_single,
        roll_probs_multi,
    };
    let start_tiles = tiles.clone();
    // fill out game_db
    r_solve(&start_tiles, &game_rules, &game_meta, &mut game_db);
    let trunk = Trunk {
        game_rules,
        game_meta,
        start_tiles,
        game_db,
    };
    println!("trunk: {:?}", trunk);
}

fn r_solve(
    tiles: &Vec<Uns>,
    game_rules: &GameRules,
    game_meta: &GameMeta,
    game_db: &mut HashMap<Vec<Uns>, GameStats>,
) -> &'static GameStats {
    let existing_game = game_db.get(game_state);
    match existing_game {
        Some(existing_game) => {
            return existing_game;
        }
        None => {
            // todo review this, weird copying for no real good reason besides simplicity to code
            if game_state.tiles.len() == 0 {
                let x = GameStats {
                    win_chance: 1.0,
                    win_chance_single: 1.0,
                    win_chance_multi: 1.0,
                    child_count: 0,
                    next_states: HashMap::new(),
                    optimal_next_states_single: HashMap::new(),
                    optimal_next_states_multi: HashMap::new(),
                };
                game_db.insert(*game_state, x);
                return &x;
            }
            let next_valid_states_hm = get_next_legal_state_hm(game_state, trphm);

            // single
            let mut next_valid_states_single = HashMap::new();
            // TODO rework to a single loop where next_valid_states_hm.get(roll) is called once per roll
            for (roll, _) in roll_probs_single {
                next_valid_states_single.insert(*roll, next_valid_states_hm.get(roll).unwrap());
            }
            let mut next_best_states_single = HashMap::new();
            for (roll, next_valid_states_single) in next_valid_states_single {
                let mut curr_roll_games = Vec::new();
                for next_valid_state_single in next_valid_states_single {
                    let next_state_single = r_solve(
                        next_valid_state_single,
                        roll_probs_multi,
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
            let mut next_valid_states_multi = HashMap::new();
            for (roll, _) in roll_probs_multi {
                next_valid_states_multi.insert(*roll, next_valid_states_hm.get(roll).unwrap());
            }
            let mut next_best_states_multi = HashMap::new();
            for (roll, next_valid_states_multi) in next_valid_states_multi {
                let mut curr_roll_games = Vec::new();
                for next_valid_state_multi in next_valid_states_multi {
                    let next_state_multi = r_solve(
                        next_valid_state_multi,
                        roll_probs_multi,
                        roll_probs_single,
                        trphm,
                        game_db,
                    );
                    curr_roll_games.push((*next_valid_state_multi, *next_state_multi));
                }
                let next_best_state_multi = get_best_game_state(curr_roll_games);
                next_best_states_multi.insert(roll, next_best_state_multi);
            }
            let win_chance_multi = get_win_chance_multi(
                game_state,
                roll_probs_multi,
                game_db,
                &next_best_states_multi,
            );

            let win_chance = win_chance_single.max(win_chance_multi);
            let child_count = next_valid_states_hm.len();
            let game_stats = GameStats {
                win_chance,
                win_chance_single,
                win_chance_multi,
                child_count,
                next_states: next_valid_states_hm,
                optimal_next_states_single: next_best_states_single,
                optimal_next_states_multi: next_best_states_multi,
            };
            game_db.insert(game_state.clone(), game_stats);
            &game_stats
        }
    }
}

fn get_best_game_state(game: Vec<(GameState, GameStats)>) -> &'static GameState<'static> {
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

fn get_single_legality(game_state: &GameState) -> bool {
    let tiles = &game_state.tiles;
    let die_vals = &game_state.die_vals;
    tiles.len() > 0 && tiles.iter().max().unwrap() <= die_vals.iter().max().unwrap()
}


fn get_next_legal_state_hm(
    game_state: &GameState,
    trphm: &HashMap<Uns, Vec<Vec<Uns>>>,
) -> HashMap<Uns, Vec<GameState>> {
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

fn get_win_chance_multi(
    game_state: &GameState,
    roll_probs_multi: &HashMap<Uns, Float>,
    game_db: &HashMap<GameState, GameStats>,
    next_best_states_multi: &HashMap<Uns, GameState>,
) -> Float {
    let tiles = &game_state.tiles;
    if tiles.len() == 0 {
        return 1.; // todo check if this is correct
    }
    let mut prob = 0.;
    for (roll, roll_prob) in roll_probs_multi {
        let best_state = next_best_states_multi.get(roll).unwrap();
        prob += roll_prob * game_db.get(best_state).unwrap().win_chance;
    }
    prob
}

fn get_win_chance_single(
    game_state: &GameState,
    roll_probs_single: &HashMap<Uns, Float>,
    game_db: &HashMap<GameState, GameStats>,
    next_best_states_single: &HashMap<Uns, GameState>,
) -> Float {
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

fn get_removed_tiles(tiles: &Vec<Uns>, trp: &Vec<Uns>) -> Option<Vec<Uns>> {
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
    let mut counts: Vec<Uns> = Vec::new();
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
 * returns an unmuted sorted vector
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
    tiles: &Vec<Uns>,
    possible_rolls: &Vec<Uns>,
    removal_max: &Uns,
) -> HashMap<Uns, Vec<Vec<Uns>>> {
    let mut trp: HashMap<Uns, Vec<Vec<Uns>>> = HashMap::new();
    for roll in possible_rolls {
        let removals: Vec<Vec<Uns>> = r_tile_removal(tiles, roll, &removal_max);
        trp.insert(*roll, removals);
    }
    trp
}

fn r_tile_removal(tiles: &[Uns], targ: &Uns, removal_max: &Uns) -> Vec<Vec<Uns>> {
    let mut removals: Vec<Vec<Uns>> = Vec::new();
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
