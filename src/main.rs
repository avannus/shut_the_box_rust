use mini_redis::{Result};
use async_recursion::async_recursion;
use rustop::opts;
use std::collections::HashMap;
use async_std::task;
use std::thread;

type Uns = u16; // any unsigned int should work, primary use of memory
type Float = f64;
type Tiles = Vec<Uns>;

#[derive(Debug)]
struct Trunk {
    game_meta: GameMeta,

    game_db: HashMap<Tiles, Float>, // contains all possible child game states
}

#[derive(Debug ,Clone)]
struct GameMeta {
    die_max: Uns, // maximum die value
    tiles: Tiles,

    trphm: HashMap<Uns, Vec<Tiles>>, // tile removal possibilities hash map
    roll_probs_single: HashMap<Uns, Float>, // probabilities of key if a single die is rolled
    roll_probs_multi: HashMap<Uns, Float>, // probabilities of key if multiple dice are rolled
}

struct InitData {
    die_vals: Vec<Uns>,
    die_cnt: Uns,
    die_max: Uns,
    start_tiles: Tiles,
    max_remove: Uns,
}

/// TODOS
/// For now, all data is being cloned for everything, which is not ideal
/// General optimizations
///     Is there a smarter way to get the game metadata, esp when sorted?
/// General Improvements
///     Allow to roll any number of dice, not just multi and single
/// Remove #[derive(Debug)] from structs
#[tokio::main]
async fn main() -> Result<()> {
    // TODO make these command line args

    let game_meta = get_game_meta();

    let game_db = split_solve(game_meta.tiles.clone(), game_meta.clone()).await;

    let trunk = Trunk { game_meta, game_db };
    println!(
        "Win chance: {:.2}%",
        trunk.game_db.get(&trunk.game_meta.tiles).unwrap() * 100.0
    );
    Ok(())
}

#[async_recursion]
async fn split_solve(tiles: Tiles, game_meta: GameMeta) -> HashMap<Tiles, Float> {
    if tiles.len() < 3 {
        let mut res = HashMap::new();
        r_solve(tiles, &game_meta, &mut res);
        return res;
    }
    let left = tiles[0..tiles.len() / 2].to_vec();
    let right = tiles[tiles.len() / 2..tiles.len()].to_vec();
    let left_meta = game_meta.clone();
    let right_meta = game_meta.clone();
    let left_future = split_solve(left, left_meta);
    let right_future = split_solve(right, right_meta);
    let left_task = task::spawn(left_future);
    let right_task = task::spawn(right_future);
    let left_res = left_task.await;
    let right_res = right_task.await;
    // join left and right
    let mut combined = left_res;
    combined.extend(right_res);
    r_solve(tiles, &game_meta, &mut combined);
    combined
}

#[allow(dead_code)]
fn get_readable_game_meta(game_meta: &GameMeta) -> String {
    let mut out = String::new();
    out.push_str(&format!("    die_max: {}\n", game_meta.die_max));
    out.push_str(&format!("    start_tiles: {:?}\n", game_meta.tiles));
    out.push_str(&format!("    trphm: {:?}\n", game_meta.trphm));
    out.push_str(&format!(
        "    roll_probs_single: {:?}\n",
        game_meta.roll_probs_single
    ));
    out.push_str(&format!(
        "    roll_probs_multi: {:?}\n",
        game_meta.roll_probs_multi
    ));
    out
}

#[allow(dead_code)]
fn get_readable_trunk_string(trunk: &Trunk) -> String {
    let mut s = String::new();
    s.push_str("  Game Meta:\n");
    s.push_str(&get_readable_game_meta(&trunk.game_meta));
    s.push_str("\nWIN CHANCE:\n");
    s.push_str(&format!(
        "  {:?}",
        trunk.game_db.get(&trunk.game_meta.tiles).unwrap()
    ));
    s.push_str("\n");
    s
}

fn r_solve(tiles: Tiles, game_meta: &GameMeta, game_db: &mut HashMap<Tiles, Float>) -> Float {
    let tiles = tiles.clone();
    let existing_game = game_db.get(&tiles);
    match existing_game {
        Some(existing_game) => {
            return existing_game.clone();
        }
        None => {
            if tiles.len() == 0 {
                let win_chance = 1.;
                game_db.insert(tiles, win_chance);
                return win_chance;
            }
            let all_next_legal_states_hm = get_next_legal_states_all(&tiles, &game_meta.trphm);
            let solved_next_legal_states_hm =
                get_all_stats_from_hm(&all_next_legal_states_hm, game_meta, game_db);

            let mut best_states_single_hm = HashMap::new();
            let mut best_states_multi_hm = HashMap::new();
            let mut win_chance_single = 0.;
            let mut win_chance_multi = 0.;
            for (roll, state_stat) in solved_next_legal_states_hm {
                let mut curr_best_win_chance = 0.;
                let mut curr_best_state: Tiles = vec![];
                for (state, win_chance) in state_stat {
                    if win_chance > curr_best_win_chance {
                        curr_best_win_chance = win_chance;
                        curr_best_state = state;
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
        if roll > &0 {
            let removals: Vec<Tiles> = r_tile_removal(tiles, roll, &removal_max);
            trp.insert(*roll, removals);
        } else {
            trp.insert(*roll, vec![vec![0]]);
        }
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

fn get_die_vals(die_min: Uns, die_max: Uns, die_val_input: Vec<Uns>) -> Vec<Uns> {
    let mut die_vals;
    if die_val_input.len() > 0 {
        die_vals = die_val_input;
    } else {
        die_vals = Vec::new();
        for i in die_min..(die_max + 1) {
            die_vals.push(i);
        }
    }
    die_vals
}

fn get_start_tiles(tile_min: Uns, tile_max: Uns, tile_input: Vec<Uns>) -> Tiles {
    let mut start_tiles;
    if tile_input.len() > 0 {
        start_tiles = tile_input;
    } else {
        start_tiles = Vec::new();
        for i in tile_min..(tile_max + 1) {
            start_tiles.push(i);
        }
    }
    start_tiles
}

fn parse_args() -> InitData {
    let (args, _) = opts! {
        synopsis "A simple example.";
        version "1.0";
        opt d_min: Uns=1, desc: "Minimum die value, increments by 1";
        opt d_max: Uns=6, desc: "Maximum die value, increments by 1";
        opt d_direct: Vec<Uns>, desc: "Die values per die, ignores min/max", multi:true;
        opt die_cnt: Uns=2, desc: "Number of dice";
        opt t_min: Uns=1, desc: "Minimum tile value, increments by 1";
        opt t_max: Uns=9, desc: "Maximum tile value, increments by 1";
        opt t_direct: Vec<Uns>, desc: "Starting tiles, ignores min_tile and max_tile", multi:true;
        opt max_remove: Uns=0, desc: "Maximum number of tiles to remove per turn, 0 for no limit";
    }
    .parse_or_exit();

    let die_vals = get_die_vals(args.d_min, args.d_max, args.d_direct);
    let die_cnt = args.die_cnt;
    let die_max = get_max(&die_vals);

    let start_tiles = get_start_tiles(args.t_min, args.t_max, args.t_direct);
    let max_remove = args.max_remove;
    InitData {
        die_vals,
        die_cnt,
        die_max,
        start_tiles,
        max_remove,
    }
}

fn get_roll_probs(die_vals: &Vec<Uns>, die_cnt: Uns, sum: Uns) -> HashMap<Uns, Float> {
    let rolls = get_srt(&get_roll_counts(die_vals, die_cnt, sum));
    get_roll_probabilities(&rolls)
}

fn get_game_meta() -> GameMeta {
    let init_data = parse_args();

    // todo probably can optimize with this sorted
    // todo eventually make this where the num dice rolled is totally dynamic
    let roll_probs_multi = get_roll_probs(&init_data.die_vals, init_data.die_cnt, 0);

    let roll_probs_single = get_roll_probs(&init_data.die_vals, 1, 0);

    let roll_possib = get_srt_dedup_keys(&roll_probs_multi, &roll_probs_single);

    let trphm =
        get_tile_removal_possibilities(&init_data.start_tiles, &roll_possib, &init_data.max_remove);

    GameMeta {
        die_max: init_data.die_max,
        trphm,
        roll_probs_single,
        roll_probs_multi,
        tiles: init_data.start_tiles,
    }
}
