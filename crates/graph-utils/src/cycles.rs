use petgraph::{
    algo::FloatMeasure,
    visit::{EdgeRef, IntoEdges},
};

pub struct NegativeCycles<G, F, K>
where
    G: IntoEdges,
    F: FnMut(G::EdgeRef) -> K,
{
    g: G,
    stack: Vec<(G::NodeId, K, G::Edges)>,
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
        let max_length = max_length.into();
        let mut s = Self {
            g,
            stack: Vec::with_capacity(max_length.unwrap_or_default()),
            path: Vec::with_capacity(max_length.unwrap_or_default()),
            edge_cost,
            max_length,
        };
        s.restart(start);
        s
    }

    pub fn restart(&mut self, start: G::NodeId) {
        self.stack.clear();
        self.stack.push((start, K::zero(), self.g.edges(start)));
        self.path.clear();
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
            let (_, cost, edges) = self.stack.last_mut()?;
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

            for (i, (node_id, last_cost, _)) in self.stack.iter().enumerate() {
                if next_edge.target() == *node_id && next_cost >= *last_cost {
                    // do not go through the same node for the second time
                    // unless next_cost is now lower
                    continue 'l;
                }
                if i > 0 {
                    let edge = self.path[i - 1];
                    if edge.id() == next_edge.id() {
                        // do not go though the same edge more than once
                        continue 'l;
                    }
                }
            }

            if next_edge.target() == self.stack[0].0 {
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

            self.path.push(next_edge);
            self.stack.push((
                next_edge.target(),
                next_cost,
                self.g.edges(next_edge.target()),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use petgraph::{
        dot::{Config, Dot},
        visit::NodeRef,
        Graph,
    };

    use super::*;

    #[test]
    fn finds_negative_cycles() {
        let mut g = Graph::new();

        let a = g.add_node("a");
        let b = g.add_node("b");
        let c = g.add_node("c");
        let d = g.add_node("d");

        let ab = g.add_edge(a, b, 7.0);
        let _ad = g.add_edge(a, d, 12.0);
        let _ba = g.add_edge(b, a, -6.0);
        let bd = g.add_edge(b, d, 3.0);
        let bc = g.add_edge(b, c, 5.0);
        let _cb = g.add_edge(c, b, -4.0);
        let cd = g.add_edge(c, d, -3.0);
        let da = g.add_edge(d, a, -11.0);
        let _db = g.add_edge(d, b, -2.0);
        let _dc = g.add_edge(d, c, 4.0);
        let da1 = g.add_edge(d, a, -12.0);

        println!(
            "{:?}",
            Dot::with_attr_getters(
                &g,
                &[Config::EdgeNoLabel, Config::NodeNoLabel],
                &|_, edge| format!("label = \"{}\"", edge.weight()),
                &|_, node| format!("label = \"{}\"", node.weight())
            )
        );

        let all = NegativeCycles::new(&g, a, |e| *e.weight(), None)
            .map(|path| path.into_iter().map(|e| e.id()).collect::<Vec<_>>())
            .collect::<HashSet<_>>();

        assert_eq!(
            all,
            [
                [ab, bc, cd, da].into(),
                [ab, bd, da1].into(),
                [ab, bc, cd, da1].into(),
                [ab, bd, da].into()
            ]
            .into_iter()
            .collect()
        );
    }
}
