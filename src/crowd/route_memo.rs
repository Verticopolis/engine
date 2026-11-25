//! The passenger routes the graph search has found since the tower last
//! changed (`Crowd::route_memo`). On one graph a search between the same two
//! segments always finds the same route, so the crowd runs it once per pair
//! and reads the answer back after that: the shoppers who come in at the
//! lobby all head to the same few venues. Keyed like `Crowd::adj` on the
//! tower revision, with the mode's search, and never saved.
use super::fasthash::FastMap;
use super::Route;

/// How many pairs the memo holds before it starts over, so a tower that
/// asks for a great many pairs between changes keeps the memo small.
const MEMO_PAIRS: usize = 8192;

#[derive(Default)]
pub struct RouteMemo {
    graph: (i64, bool),
    pairs: usize,
    routes: FastMap<(i64, i64), Option<Route>>,
}

impl RouteMemo {
    /// The route found from `from` to `to` on `graph`, the tower revision
    /// and whether the search kept the walk budget (`None` inside when there
    /// was none), or `None` when it is not known.
    pub fn get(&mut self, graph: (i64, bool), from: i64, to: i64) -> Option<&Option<Route>> {
        if self.graph != graph {
            self.routes.clear();
            self.pairs = 0;
            self.graph = graph;
        }
        self.routes.get(&(from, to))
    }

    /// Keeps the route `get` did not know.
    pub fn put(&mut self, from: i64, to: i64, route: &Option<Route>) {
        if self.pairs >= MEMO_PAIRS {
            self.routes.clear();
            self.pairs = 0;
        }
        self.routes.insert((from, to), route.clone());
        self.pairs += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(f: i64) -> Option<Route> {
        Some(Route {
            floors: vec![1, f],
            shafts: vec![7],
        })
    }

    #[test]
    fn a_new_revision_forgets_the_old_routes() {
        let mut memo = RouteMemo::default();
        assert!(memo.get((3, true), 1, 2).is_none());
        memo.put(1, 2, &r(2));
        assert_eq!(
            memo.get((3, true), 1, 2).unwrap().as_ref().unwrap().floors,
            vec![1, 2]
        );
        memo.put(1, 4, &None);
        assert!(
            memo.get((3, true), 1, 4).unwrap().is_none(),
            "no route is kept too"
        );
        assert!(memo.get((3, false), 1, 2).is_none(), "the other search");
        memo.put(1, 2, &r(2));
        assert!(memo.get((4, false), 1, 2).is_none(), "a new revision");
    }

    #[test]
    fn a_full_memo_starts_over() {
        let mut memo = RouteMemo::default();
        memo.get((1, true), 0, 0);
        for i in 0..MEMO_PAIRS as i64 {
            memo.put(0, i, &r(i));
        }
        assert!(memo.get((1, true), 0, 5).is_some());
        memo.put(1, 1, &r(1));
        assert!(memo.get((1, true), 0, 5).is_none());
        assert!(memo.get((1, true), 1, 1).is_some());
    }
}
