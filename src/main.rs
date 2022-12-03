extern crate rustop;

use std::{collections::HashMap};

fn sort_unstable<T: Clone + Ord>(a: &[T]) -> Vec<T> {
    let mut b = a.to_vec();
    b.sort_unstable();
    b
}

fn get_sorted_deduped_roll_possibilities(hm1: &HashMap<u8, f64>, hm2: &HashMap<u8, f64>) -> Vec<u8> {
    let mut x = hm1.keys().map(|&x| x).collect::<Vec<u8>>();
    x.append(&mut hm2.keys().map(|&x| x).collect::<Vec<u8>>());
    x.dedup();
    sort_unstable(&x)
}

fn main() {
    let die_vals: Vec<u8> = [1, 2, 3, 4, 5, 6].to_vec();
    let die_cnt: u8 = 2;
    let tiles: Vec<u8> = sort_unstable(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let max_remove: u8 = 0;

    // todo probably can optimize with this sorted
    let roll_cnts = sort_unstable(&get_roll_counts(&die_vals, die_cnt, 0));
    let roll_probs = get_roll_probabilities(&roll_cnts);
    let roll_probs_single = get_roll_probabilities_single(&die_vals);
    // rolls are sorted
    let roll_possib = get_sorted_deduped_roll_possibilities(&roll_probs, &roll_probs_single);
    let trphm = get_tile_removal_possibilities(&tiles, &roll_possib, &max_remove);
    println!("roll_counts: {:?}", roll_cnts);
    println!("possible_rolls: {:?}", roll_possib);
    println!("roll_probabilities: {:?}", roll_probs);
    println!("roll_probabilities_single: {:?}", roll_probs_single);
    println!("trphm: {:?}", trphm);

    let ans = r_solve(&tiles, &roll_probs, &trphm, get_single_legality(&tiles, &die_vals), &die_vals, &roll_probs_single);
    println!("{}", ans);
}

fn get_roll_probabilities_single(die_vals: &Vec<u8>) -> HashMap<u8, f64> {
    let mut roll_probs = HashMap::new();
    for &die_val in die_vals {
        roll_probs.insert(die_val, 1.0 / die_vals.len() as f64);
    }
    roll_probs
}

fn get_single_legality(tiles: &Vec<u8>, die_vals: &Vec<u8>) -> bool {
    tiles.len() > 0 && tiles.iter().max().unwrap() <= die_vals.iter().max().unwrap()
}

fn r_solve(
    tiles: &Vec<u8>,
    roll_probs: &HashMap<u8, f64>,
    trphm: &HashMap<u8, Vec<Vec<u8>>>,
    single_is_legal: bool,
    die_vals: &Vec<u8>,
    roll_probs_single: &HashMap<u8, f64>,
) -> f64 {
    let win_single = get_win_chance_single(tiles, roll_probs, trphm, single_is_legal, die_vals, roll_probs_single);
    let win_double = get_win_chance_double(tiles, roll_probs, trphm, die_vals, roll_probs_single);
    win_single.max(win_double)
}

fn get_win_chance_double(
    tiles: &Vec<u8>,
    roll_probs: &HashMap<u8, f64>,
    trphm: &HashMap<u8, Vec<Vec<u8>>>,
    die_vals: &Vec<u8>,
    roll_probs_single: &HashMap<u8, f64>,
) -> f64 {
    if tiles.len() == 0 {
        return 1.;
    }
    let mut prob = 0.;
    for (roll, roll_prob) in roll_probs {
        let trps = trphm.get(roll).unwrap();
        let mut rolls: Vec<f64> = Vec::new();
        for trp in trps {
            let new_tiles = get_removed_tiles(tiles, trp);
            match new_tiles {
                Some(new_tiles) => {
                    let curr_prob = roll_prob;
                    rolls.push(curr_prob * r_solve(
                        &new_tiles,
                        roll_probs,
                        trphm,
                        get_single_legality(&new_tiles, &die_vals),
                        die_vals,
                        roll_probs_single,
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

fn get_win_chance_single(
    tiles: &Vec<u8>,
    roll_probs: &HashMap<u8, f64>,
    trphm: &HashMap<u8, Vec<Vec<u8>>>,
    single_is_legal: bool,
    die_vals: &Vec<u8>,
    roll_probs_single: &HashMap<u8, f64>,
) -> f64 {
    if tiles.len() == 0 {
        return 1.;
    }
    if !single_is_legal {
        return 0.;
    }
    let mut prob = 0.;
    for (roll, roll_prob) in roll_probs_single {
        let trps = trphm.get(roll).unwrap();
        let mut rolls: Vec<f64> = Vec::new();
        for trp in trps {
            let new_tiles = get_removed_tiles(tiles, trp);
            match new_tiles {
                Some(new_tiles) => {
                    let curr_prob = roll_prob;
                    rolls.push(curr_prob * r_solve(
                        &new_tiles,
                        roll_probs,
                        trphm,
                        get_single_legality(&new_tiles, &die_vals),
                        die_vals,
                        roll_probs_single,
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
