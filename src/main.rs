use rayon::prelude::*;
use rustop::opts;
use std::collections::HashMap;

/// The unsigned int type used for all non-usize int calculations
type Uns = u16;
/// The float type used for all float calculations
type Float = f64;
/// Helper type alias for a vector of tile values, for readability
type Tiles = Vec<Uns>;

/// The parent of a given game containing all data from solving the game
#[derive(Debug)]
struct Trunk {
    game_meta: GameMeta,

    game_db: HashMap<Tiles, Float>, // contains all possible child game states
}

// TODO allow multiple algos?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Algorithm {
    All,
    Naive,
    Depth,
    Parallel,
    Default,
}

/// Minimal necessary data to calculate a given game. Shared between all game states of a game.
#[derive(Debug, Clone)]
struct GameMeta {
    /// The maximum value of the die
    die_max: Uns,
    /// The starting tiles
    tiles: Tiles,

    /// Tile Removal Possibilities Hash Map, key: roll, value: Vec of tile combinations to remove.
    /// This is calculated given the starting tiles, the die sides, and the max number of tiles to remove
    trphm: HashMap<Uns, Vec<Tiles>>,
    /// Perfect play win chance if a single die is rolled (0 if not legal)
    roll_probs_single: HashMap<Uns, Float>,
    /// Perfect play win chance if multiple dice are rolled (always legal)
    roll_probs_multi: HashMap<Uns, Float>,

    /// The algorithm to use for solving
    algorithm: Algorithm,
}

/// Data extracted from program args (or lack thereof)
struct InitData {
    /// The sides of the given die
    die_vals: Vec<Uns>,
    /// The number of dice
    die_cnt: Uns,
    /// The starting tiles (trunk)
    start_tiles: Tiles,
    /// The maximum number of tiles to remove on a given turn
    max_remove: Uns,
    /// Run all algos
    algorithm: Algorithm,
}

/// Solves a given game.
/// TODOS:
/// For now, all data is being cloned for everything, which is not ideal
/// General optimizations
///     Is there a smarter way to get the game metadata, esp when sorted?
/// General Improvements
///     Allow to roll any number of dice, not just multi and single
/// Remove #[derive(Debug)] from structs
fn main(){
    // TODO make these command line args

    let game_meta = get_game_meta();
    println!("Done with setup, solving game states...");
    
    // time this function
    let start = std::time::Instant::now();
    let algorithm = if game_meta.algorithm == Algorithm::Default {
        Algorithm::Parallel
    } else {
        game_meta.algorithm
    };

    if algorithm == Algorithm::All || algorithm == Algorithm::Naive {
        println!("Solving with naive algorithm...");
        let start = std::time::Instant::now();
        let naive_prob = naive_solve(game_meta.tiles.clone(), &game_meta);
        println!("Win chance: {:.2}%", naive_prob * 100.0);
        // println!("num of game entries: {}", naive_prob.len());
        let duration = start.elapsed().as_secs_f64();
        println!("Time elapsed in naive_solve() is: {:.3}s\n", duration);
    }
    if algorithm == Algorithm::All || algorithm == Algorithm::Depth {
        println!("Solving with depth algorithm...");
        let start = std::time::Instant::now();
        let mut depth_db = HashMap::new();
        depth_solve(game_meta.tiles.clone(), &game_meta, &mut depth_db);
        println!("Win chance: {:.2}%", depth_db.get(&game_meta.tiles).unwrap() * 100.0);
        println!("num of game entries: {}", depth_db.len());
        let duration = start.elapsed().as_secs_f64();
        println!("Time elapsed in depth_solve() is: {:.3}s\n", duration);
    }
    if algorithm == Algorithm::All || algorithm == Algorithm::Parallel {
        println!("Solving with parallel algorithm...");
        let start = std::time::Instant::now();
        let par_db = par_solve(game_meta.tiles.clone(), game_meta.clone());
        println!("Win chance: {:.2}%", par_db.get(&game_meta.tiles).unwrap() * 100.0);
        println!("num of game entries: {}", par_db.len());
        let duration = start.elapsed().as_secs_f64();
        println!("Time elapsed in par_solve() is: {:.3}s\n", duration);
    }
    
    let duration = start.elapsed().as_secs_f64();
    println!("Total time elapsed is: {:.3}s\n", duration);
    // println!("{:?}", &trunk.game_meta.trphm);
}

/// Gets all combinations of remaining tiles, ordered by increasing number of tiles remaining
/// TODO probably doesn't work when there are gaps in number of tiles remaining
fn get_tile_combos(tiles: &Tiles) -> Vec<Vec<Tiles>> {
    let mut game_states = Vec::new();
    for i in 1..tiles.len() {
        game_states.push(get_game_states_by_tiles_remaining(tiles, &Vec::new(), i));
    }
    game_states
}

/// Gets all possible game states with a given start condition and a number of tiles remaining
fn get_game_states_by_tiles_remaining(remaining_tiles: &Tiles, curr_tiles: &Tiles, num_tiles: usize) -> Vec<Tiles> {
    // TODO async?
    if num_tiles == curr_tiles.len() {
        return vec![curr_tiles.clone()];
    }
    let mut sol = Vec::new();
    for i in 0..remaining_tiles.len() {
        if curr_tiles.len() > 0 && remaining_tiles[i] < curr_tiles[curr_tiles.len() - 1] {
            continue;
        }
        let mut curr_tiles = curr_tiles.clone();
        let mut remaining_tiles = remaining_tiles.clone();
        curr_tiles.push(remaining_tiles.remove(i));
        let sols = get_game_states_by_tiles_remaining(&remaining_tiles, &curr_tiles, num_tiles);
        sol.extend(sols);
    }
    sol
}

/// Solves a given game in parallel
fn par_solve(tiles: Tiles, game_meta: GameMeta) -> HashMap<Tiles, Float> {
    let mut result = HashMap::new();

    print!("about to get tile combos... ");
    let t_combos = get_tile_combos(&tiles);
    println!("DONE");

    for t_combo in t_combos {
        // let tiles = tiles.clone();
        // let chunks = tiles.chunks(chunk_size.max(1));
        // let vec = chunks.collect::<Vec<_>>();
        let vec: Vec<Tiles> = t_combo;
        result = result.clone();
        // if vec.len() == 45 {
        //     println!("Curr res for combos {:?} :\n{:?}", vec, result);
        // }
        let par_iter = vec
            .par_iter()
            // .filter_map(|value| value.as_ref().ok())
            .map(|chunk| {
                let mut res = result.clone();
                depth_solve(chunk.to_vec(), &game_meta, &mut res);
                res
            })
            .reduce(
                || HashMap::new(),
                |m1, m2| {
                    let mut res = m1;
                    res.extend(m2);
                    res
                },
            );
        println!("par_iter len: {:?}", par_iter.len());
        result.extend(par_iter);
    }
    depth_solve(tiles, &game_meta, &mut result);
    result
}

/// Returns a readable String of the given GameMeta
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

/// Returns a readable String of the given Trunk
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

/// Recursively and naively solves a given game through a breadth-first traversal
fn naive_solve(tiles: Tiles, game_meta: &GameMeta) -> Float {
    let win_single = naive_solve_single(tiles.clone(), game_meta);
    let win_multi = naive_solve_multi(tiles.clone(), game_meta);
    win_single.max(win_multi)
}

fn naive_solve_single(tiles: Tiles, game_meta: &GameMeta) -> Float {
    if tiles.len() == 0 {
        return 1.;
    }
    let mut prob = 0.;
    let roll_probs = &game_meta.roll_probs_single;
    let trphm = &game_meta.trphm;
    for (roll, roll_prob) in roll_probs {
        let trps = trphm.get(roll).unwrap();
        let mut rolls = Vec::new();
        for trp in trps {
            let new_tiles = get_removed_tiles(&tiles, trp);
            match new_tiles {
                Some(new_tiles) => {
                    let curr_prob = roll_prob;
                    rolls.push(curr_prob * naive_solve(
                        new_tiles,
                        game_meta,
                    ));
                }
                None => {}
            }
        }
        if rolls.len() > 0 {
            prob += rolls.iter().cloned().fold(0. / 0., f64::max);
        }
    }
    prob
}

fn naive_solve_multi(tiles: Tiles, game_meta: &GameMeta) -> Float {
    if tiles.len() == 0 {
        return 1.;
    }
    let mut prob = 0.;
    let roll_probs = &game_meta.roll_probs_multi;
    let trphm = &game_meta.trphm;
    for (roll, roll_prob) in roll_probs {
        let trps = trphm.get(roll).unwrap();
        let mut rolls = Vec::new();
        for trp in trps {
            let new_tiles = get_removed_tiles(&tiles, trp);
            match new_tiles {
                Some(new_tiles) => {
                    let curr_prob = roll_prob;
                    rolls.push(curr_prob * naive_solve(
                        new_tiles,
                        game_meta,
                    ));
                }
                None => {}
            }
        }
        if rolls.len() > 0 {
            prob += rolls.iter().cloned().fold(0. / 0., f64::max);
        }
    }
    prob
}

/// Recursively solves a given game through a depth-first traversal
fn depth_solve(tiles: Tiles, game_meta: &GameMeta, game_db: &mut HashMap<Tiles, Float>) -> Float {
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

/// Returns a vec of tile possibilities for the next turn given a Tile Removal Possibilities for a given roll.
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

/// Returns a HashMap of all next legal states where key: roll, value: vec of tile possibilities for the next turn
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

/// TODO
fn get_all_stats_from_states(
    states: &Vec<Tiles>,
    game_meta: &GameMeta,
    game_db: &mut HashMap<Tiles, Float>,
) -> Vec<(Tiles, Float)> {
    let mut res = Vec::new();
    for state in states {
        let state = state.clone();
        let stats = depth_solve(state.clone(), game_meta, game_db);
        res.push((state, stats));
    }
    res
}

/// TODO
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

/// Returns a bool of the legality of rolling a single die
fn get_single_legality(tiles: &Tiles, max_die: &Uns) -> bool {
    tiles.len() > 0 && tiles.iter().max().unwrap() <= max_die
}

/// Returns the maximum value within a Vec<Uns>, used to find max die value
fn get_max(vals: &[Uns]) -> Uns {
    vals.iter().max().unwrap().clone()
}

/// If the given tiles can be removed, returns the new tiles, otherwise returns None
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

/// Returns a sorted vector from an unsorted vector
fn get_srt<T: Copy + Ord>(a: &[T]) -> Vec<T> {
    let mut b = a.to_vec();
    b.sort_unstable();
    b
}

/// Returns a sorted deduplicated vector from an unsorted vector
fn get_srt_dedup_keys<T, U>(hm1: &HashMap<Uns, T>, hm2: &HashMap<Uns, U>) -> Vec<Uns> {
    let mut x = hm1.keys().map(|&x| x).collect::<Vec<Uns>>();
    x.append(&mut hm2.keys().map(|&x| x).collect::<Vec<Uns>>());
    x = get_srt(&x);
    x.dedup();
    x
}

/// Returns a HashMap of all combinations of tiles to remove from a given roll
/// key: roll, value: Vec<Tiles>, where Tiles is the combination of tiles to remove
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

/// Recursive function that returns all possible combinations of tiles to remove from a given roll
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

/// Creates a Vec<die values> given a min and max
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

/// Creates a Vec<tile values>=Tiles given a min and max
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

/// Parses command line arguments and returns them as a calculated struct
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
        opt all: bool=false, desc: "Run using all possible algorithms";
        opt naive: bool=false, desc: "Run using naive algorithm";
        opt depth: bool=false, desc: "Run using depth first search singly-threaded algorithm";
        opt parallel: bool=false, desc: "Run using parallel algorithm";
    }
    .parse_or_exit();

    let die_vals = get_die_vals(args.d_min, args.d_max, args.d_direct);
    let die_cnt = args.die_cnt;

    let start_tiles = get_start_tiles(args.t_min, args.t_max, args.t_direct);
    let max_remove = args.max_remove;

    let algorithm = if args.all {
        Algorithm::All
    } else if args.naive {
        Algorithm::Naive
    } else if args.depth {
        Algorithm::Depth
    } else if args.parallel {
        Algorithm::Parallel
    } else {
        Algorithm::Default
    };

    InitData {
        die_vals,
        die_cnt,
        start_tiles,
        max_remove,
        algorithm,
    }
}

/// Returns a Hashmap of all possible rolls and their probabilities given some die_vals and die_cnt
fn get_roll_probs(die_vals: &Vec<Uns>, die_cnt: Uns, sum: Uns) -> HashMap<Uns, Float> {
    let rolls = get_srt(&get_roll_counts(die_vals, die_cnt, sum));
    get_roll_probabilities(&rolls)
}

/// Returns a vector such that each element is a roll of the dice, repeating the sums as many times as they are rolled
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

/// Returns a HashMap of all possible rolls and their probabilities given a sorted Vec of rolls
/// key: roll, value: probability
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

fn get_game_meta() -> GameMeta {
    let init_data = parse_args();

    let die_max = get_max(&init_data.die_vals);

    // todo probably can optimize with this sorted
    // todo eventually make this where the num dice rolled is totally dynamic
    let roll_probs_multi = get_roll_probs(&init_data.die_vals, init_data.die_cnt, 0);

    let roll_probs_single = get_roll_probs(&init_data.die_vals, 1, 0);

    let roll_possib = get_srt_dedup_keys(&roll_probs_multi, &roll_probs_single);

    let trphm =
        get_tile_removal_possibilities(&init_data.start_tiles, &roll_possib, &init_data.max_remove);


    GameMeta {
        die_max,
        trphm,
        roll_probs_single,
        roll_probs_multi,
        tiles: init_data.start_tiles,
        algorithm: init_data.algorithm,
    }
}
