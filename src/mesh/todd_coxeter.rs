use std::collections::{HashMap, HashSet, VecDeque};

struct CosetTable {
    num_gens: usize,
    table: Vec<Vec<Option<usize>>>,
    live_map: Vec<usize>,
}

impl CosetTable {
    fn init(num_gens: usize) -> Self {
        let table = vec![vec![None; num_gens]];
        let live_map = vec![0];

        CosetTable {
            num_gens,
            table,
            live_map,
        }
    }

    fn define(&mut self, coset: usize, g: usize) {
        let fresh = self.table.len();
        self.table.push(vec![None; self.num_gens]);
        self.table[coset][g] = Some(fresh);
        self.table[fresh][g] = Some(coset);
        self.live_map.push(fresh);
    }

    fn rep(&mut self, coset: usize) -> usize {
        let mut m = coset;
        while m != self.live_map[m] {
            m = self.live_map[m];
        }

        let mut j = coset;
        while j != self.live_map[j] {
            let next = self.live_map[j];
            self.live_map[j] = m;
            j = next;
        }

        m
    }

    fn merge(&mut self, queue: &mut Vec<usize>, coset1: usize, coset2: usize) {
        let (s, t) = (self.rep(coset1), self.rep(coset2));
        if s != t {
            let (s, t) = (s.min(t), s.max(t));
            self.live_map[t] = s;
            queue.push(t);
        }
    }

    fn coincidence(&mut self, coset1: usize, coset2: usize) {
        let mut queue = Vec::new();

        self.merge(&mut queue, coset1, coset2);
        while queue.len() != 0 {
            let e = queue.remove(0);
            for g in 0..self.num_gens {
                if let Some(f) = self.table[e][g] {
                    self.table[f][g] = None;

                    let (e_, f_) = (self.rep(e), self.rep(f));
                    if let Some(x) = self.table[e_][g] {
                        self.merge(&mut queue, f_, x);
                    } else if let Some(x) = self.table[f_][g] {
                        self.merge(&mut queue, e_, x);
                    } else {
                        self.table[e_][g] = Some(f_);
                        self.table[f_][g] = Some(e_);
                    }
                }
            }
        }
    }

    fn scan_and_fill(&mut self, coset: usize, word: &[usize]) {
        let mut f = coset;
        let mut b = coset;
        let mut i = 0;
        let mut j = word.len() - 1;

        loop {
            // scan forwards as far as possible
            while i <= j && self.table[f][word[i]].is_some() {
                f = self.table[f][word[i]].unwrap();
                i += 1;
            }

            if i > j {
                if f != b {
                    // found a coincidence
                    self.coincidence(f, b);
                }
                return;
            }

            while j >= i && self.table[b][word[j]].is_some() {
                b = self.table[b][word[j]].unwrap();
                j -= 1;
            }

            if j < i {
                self.coincidence(f, b);
                return;
            } else if i == j {
                // deduction
                self.table[f][word[i]] = Some(b);
                self.table[b][word[i]] = Some(f);
            } else {
                // define a new coset, continue scanning
                self.define(f, word[i]);
            }
        }
    }
}

pub fn coset_table(
    num_gens: usize,
    relations: &[&[usize]],
    sub_gens: &[usize],
) -> Vec<Vec<usize>> {
    let mut table = CosetTable::init(num_gens);

    // fill in initial information for the first coset
    for g in sub_gens {
        table.scan_and_fill(0, &[*g]);
    }

    let mut i = 0;
    while i < table.table.len() {
        if table.live_map[i] == i {
            for rel in relations {
                table.scan_and_fill(i, rel);
            }

            for g in 0..num_gens {
                if table.table[i][g].is_none() {
                    table.define(i, g);
                }
            }
        }

        i += 1;
    }

    // compress the resulting table
    let mut forward_map = HashMap::new();
    let mut backward_map = HashMap::new();
    let mut fresh = 0;
    for coset in 0..table.table.len() {
        if table.live_map[coset] == coset {
            forward_map.insert(coset, fresh);
            backward_map.insert(fresh, coset);
            fresh += 1;
        }
    }

    let mut compressed_table = Vec::new();
    for i in 0..fresh {
        compressed_table.push(
            table.table[backward_map[&i]]
                .iter()
                .map(|x| forward_map[&x.unwrap()])
                .collect(),
        );
    }

    compressed_table
}

pub fn coset_table_bfs(
    table: &Vec<Vec<usize>>,
    start: usize,
) -> Vec<Vec<usize>> {
    let mut paths = vec![Vec::new(); table.len()];
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();

    queue.push_back(start);
    seen.insert(start);

    while let Some(top) = queue.pop_front() {
        for (g, next) in table[top].iter().enumerate() {
            if seen.contains(&next) {
                continue;
            }

            paths[*next] = paths[top].clone();
            paths[*next].push(g);
            queue.push_back(*next);
            seen.insert(*next);
        }
    }

    paths
}

pub fn table_bfs_fold<T, F>(
    table: &Vec<Vec<usize>>,
    start: usize,
    initial: T,
    f: F,
) -> Vec<T>
where
    T: Clone,
    F: Fn(T, usize) -> T,
{
    let mut result = vec![initial.clone(); table.len()];
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();

    queue.push_back(start);
    seen.insert(start);

    while let Some(top) = queue.pop_front() {
        for (g, next) in table[top].iter().enumerate() {
            if seen.contains(&next) {
                continue;
            }

            result[*next] = f(result[top].clone(), g);
            queue.push_back(*next);
            seen.insert(*next);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dodecahedron_test() {
        let table = coset_table(
            4,
            &[
                &[0, 0],
                &[1, 1],
                &[2, 2],
                &[3, 3],
                &[0, 1].repeat(5),
                &[1, 2].repeat(3),
                &[2, 3].repeat(3),
                &[0, 2].repeat(2),
                &[0, 3].repeat(2),
                &[1, 3].repeat(2),
            ],
            &[0, 1, 3],
        );
        println!("{:?}", table);

        let directions = coset_table_bfs(&table, 0);
        println!("{:?}", directions);
    }
}
