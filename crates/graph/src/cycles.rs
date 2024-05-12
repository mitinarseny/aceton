use petgraph::{
    algo::FloatMeasure,
    visit::{EdgeRef, GraphBase, IntoEdgeReferences, IntoEdges},
};

pub struct NegativeCycles<G, F, K>
where
    G: IntoEdges,
    F: FnMut(G::EdgeRef) -> K,
{
    g: G,
    start: G::NodeId,
    stack: Vec<(K, G::Edges)>,
    path: Vec<G::EdgeRef>,
    edge_cost: F,
    max_length: Option<usize>,
}

impl<G, F, K> NegativeCycles<G, F, K>
where
    G: IntoEdges,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    pub fn new(g: G, start: G::NodeId, edge_cost: F, max_length: impl Into<Option<usize>>) -> Self {
        Self {
            g,
            start,
            stack: [(K::zero(), g.edges(start))].into(),
            path: Vec::new(),
            edge_cost,
            max_length: max_length.into(),
        }
    }
}

impl<G, F, K> Iterator for NegativeCycles<G, F, K>
where
    G: IntoEdges,
    F: FnMut(G::EdgeRef) -> K,
    K: FloatMeasure,
{
    type Item = Vec<G::EdgeRef>;

    fn next(&mut self) -> Option<Self::Item> {
        'l: loop {
            let (cost, edges) = self.stack.last_mut()?;
            let Some(next_edge) = edges.next() else {
                self.path.pop();
                self.stack.pop();
                continue;
            };

            let next_cost = *cost + (self.edge_cost)(next_edge);
            // TODO
            // if next_cost == K::infinite() {
            //     continue;
            // }

            if next_edge.target() == self.start {
                if next_cost >= K::zero() {
                    continue;
                }
                let mut cycle = self.path.clone();
                cycle.push(next_edge);
                return Some(cycle);
            }

            if self
                .max_length
                .is_some_and(|max_length| self.path.len() == max_length - 1)
            {
                continue;
            }

            if self.path.iter().any(|e| e.id() == next_edge.id()) {
                // do not visit already visited
                continue 'l;
            }

            self.path.push(next_edge);
            self.stack
                .push((next_cost, self.g.edges(next_edge.target())));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use petgraph::{dot::Dot, Directed, Graph};

    use super::*;

    #[test]
    fn finds_all_cycles() {
        let mut g = Graph::new();

        let a = g.add_node("a");
        let b = g.add_node("b");
        let c = g.add_node("c");
        let d = g.add_node("d");
        let e = g.add_node("e");
        let f = g.add_node("f");

        let ab = g.add_edge(a, b, "ab");
        let af = g.add_edge(a, f, "af");
        let ba = g.add_edge(b, a, "ba");
        let bc = g.add_edge(b, c, "bc");
        let be = g.add_edge(b, e, "be");
        let bf = g.add_edge(b, f, "bf");
        let cb = g.add_edge(c, b, "cb");
        let cd = g.add_edge(c, d, "cd");
        let ce = g.add_edge(c, e, "ce");
        let eb = g.add_edge(e, b, "eb");
        let ec = g.add_edge(e, c, "ec");
        let ef = g.add_edge(e, f, "ef");
        let fa = g.add_edge(f, a, "fa");
        let fb = g.add_edge(f, b, "fb");
        let fe = g.add_edge(f, e, "fe");
        let fa1 = g.add_edge(f, a, "fa1");

        println!("{:?}", Dot::new(&g));

        let all = NegativeCycles::new(&g, a, |_| -1.0, None)
            .map(|path| path.into_iter().map(|e| e.weight()).collect::<Vec<_>>())
            .collect::<HashSet<_>>();

        println!("all: {all:?}");
    }
}
