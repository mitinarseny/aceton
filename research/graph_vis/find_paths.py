from graph import create_graph
import networkx as nx
from itertools import permutations


def remove_leaf_nodes(G: nx.Graph) -> nx.Graph:
    H = G.copy()
    leaf_nodes = [node for node, degree in dict(H.degree()).items() if degree == 2]
    H.remove_nodes_from(leaf_nodes)
    print(f"{len(G.nodes())} -> {len(H.nodes())}")
    return H


def find_arbitrage_paths(graph: nx.Graph, start_node: str, path_length: int) -> list:
    if path_length < 2:
        return []

    paths = [
        (start_node, *p, start_node)
        for p in permutations(
            [n for n in graph.nodes if n != start_node], path_length - 1
        )
    ]

    arbitrage_paths = []
    for path in paths:
        if all(graph.has_edge(path[i], path[i + 1]) for i in range(len(path) - 1)):
            # Calculate products for numerator and denominator
            product_numerator = 1
            product_denominator = 1
            total_commission = 1
            reserves = []

            for i in range(len(path) - 1):
                edge_data = graph.get_edge_data(path[i], path[i + 1])
                product_numerator *= edge_data["reserve0"]
                product_denominator *= edge_data["reserve1"]
                commission_rate = edge_data["fee"]
                total_commission *= commission_rate**2
                reserves.append((edge_data["reserve1"], edge_data["reserve0"]))

            if product_denominator == 0:
                continue
            if product_numerator / product_denominator > 1 / total_commission:
                arbitrage_paths.append(
                    (path, product_numerator / product_denominator, reserves)
                )

    return arbitrage_paths


def calculate_profit_symbolic(reserves, amount_in):
    numerator = reserves[0][0] * reserves[1][1] * reserves[2][1] * amount_in
    denominator = (
        reserves[0][1] * reserves[1][0] * reserves[2][0]
        + reserves[1][1] * reserves[2][0] * amount_in
        + reserves[1][1] * reserves[2][1] * amount_in
        + reserves[1][0] * reserves[2][0] * amount_in
    )
    return numerator / denominator


if __name__ == "__main__":

    print("Creating graph...")
    graph = create_graph()
    print("Removing leaf nodes...")
    graph = remove_leaf_nodes(graph)
    print("Finding arbitrage opportunities...")
    arbitrage_opportunities = find_arbitrage_paths(
        graph, "EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c", 3
    )

    for path, product_of_rates, reserves in arbitrage_opportunities:
        path = [
            (
                "TON"
                if node == "EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c"
                else node
            )
            for node in path
        ]
        str_path = " -> ".join(path)
        profit = calculate_profit_symbolic(reserves, 1)
        print("-" * 80)
        print(f"Path: {str_path}, Product of Rates: {product_of_rates:.4f}")
        print(f"Reserves: {reserves}")
        print(f"Profit: {profit:.4f}")
