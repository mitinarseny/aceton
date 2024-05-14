from graph import create_graph
import networkx as nx
from itertools import permutations
import numpy as np
import pandas as pd
import requests as req
import datetime

TON_AMOUNT = 1


def remove_leaf_nodes(G: nx.Graph) -> nx.Graph:
    H = G.copy()
    leaf_nodes = [node for node, degree in dict(H.degree()).items() if degree == 2]
    H.remove_nodes_from(leaf_nodes)
    print(f"{len(G.nodes())} -> {len(H.nodes())}")
    return H


def find_arbitrage_paths(
    graph: nx.Graph, start_node: str, path_length: int, trade_amount: float
) -> list:
    if path_length < 2:
        return []

    trade_amount = trade_amount * 10**9

    paths = [
        (start_node, *p, start_node)
        for p in permutations(
            [n for n in graph.nodes if n != start_node], path_length - 1
        )
    ]

    arbitrage_paths = []
    for path in paths:
        if all(graph.has_edge(path[i], path[i + 1]) for i in range(len(path) - 1)):
            product_numerator = 1
            product_denominator = 1
            reserves = []
            slippages = []

            for i in range(len(path) - 1):
                edge_data = graph.get_edge_data(path[i], path[i + 1])
                reserve0, reserve1 = edge_data["reserve0"], edge_data["reserve1"]
                if reserve0 == 0 or reserve1 == 0:
                    reserves = []
                    break

                product_numerator *= reserve0
                product_denominator *= reserve1
                reserves.append((reserve0, reserve1))
                slippages.append(calculate_slippage((reserve0, reserve1), trade_amount))

            if product_denominator == 0 or not reserves:
                continue

            positive_slippage_condition = all(slippage > 0.8 for slippage in slippages)

            if (
                product_numerator / product_denominator > 1
                and positive_slippage_condition
            ):
                arbitrage_paths.append(
                    {
                        "path": [
                            (
                                "TON"
                                if node
                                == "EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c"
                                else node
                            )
                            for node in path
                        ],
                        "product_of_rates": product_numerator / product_denominator,
                        "reserves": reserves,
                        "slippages": slippages,
                    }
                )

    return arbitrage_paths


def calculate_slippage(reserves, amount_in):
    perfect_exchange = reserves[0] / reserves[1]
    actual_exchange = reserves[1] - reserves[0] * reserves[1] / (
        reserves[0] + amount_in
    )
    slippage = max((perfect_exchange - actual_exchange) / perfect_exchange, 0)

    return round(slippage, 4)


def calculate_profit_symbolic(reserves, amount_in):
    amount_in = amount_in * 10**9
    numerator = reserves[0][0] * reserves[1][1] * reserves[2][1] * amount_in
    denominator = (
        reserves[0][1] * reserves[1][0] * reserves[2][0]
        + reserves[1][1] * reserves[2][0] * amount_in
        + reserves[1][1] * reserves[2][1] * amount_in
        + reserves[1][0] * reserves[2][0] * amount_in
    )
    return numerator / denominator


def main():
    print("Creating graph...")
    graph = create_graph()
    print("Removing leaf nodes...")
    graph = remove_leaf_nodes(graph)
    print("Finding arbitrage opportunities...")
    arbitrage_opportunities = find_arbitrage_paths(
        graph,
        "EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c",
        3,
        TON_AMOUNT,
    )

    for opportunity in arbitrage_opportunities:
        path = opportunity["path"]
        str_path = " -> ".join(path)
        profit = calculate_profit_symbolic(opportunity["reserves"], TON_AMOUNT)

        print("-" * 80)
        print(
            f"Path: {str_path}, Product of Rates: {opportunity['product_of_rates']:.4f}"
        )
        print(f"Reserves: {opportunity['reserves']}")
        print(f"Slippages per Hop: {opportunity['slippages']}")
        print(f"Profit: {profit:.4f}")
    print("-" * 80)
    print(f"Total paths evaluated: {len(arbitrage_opportunities)}")
    return arbitrage_opportunities


def get_pool_trades(pool):
    url = f"https://api.dedust.io/v2/pools/{pool}/trades"
    data = req.get(url).json()
    amount_ins = np.array([int(x["amountIn"]) for x in data])
    amount_outs = np.array([int(x["amountOut"]) for x in data])
    traders = np.unique(np.array([x["sender"] for x in data]))
    times = np.array(
        [
            datetime.datetime.strptime(x["createdAt"], "%Y-%m-%dT%H:%M:%S.%fZ")
            for x in data
        ]
    )
    times = np.sort(times)
    number_of_trades = len(data)
    return amount_ins, amount_outs, traders, times, number_of_trades


def pool_stats(pool):
    amount_ins, amount_outs, traders, times, number_of_trades = get_pool_trades(pool)
    mean_in = np.mean(amount_ins)
    mean_out = np.mean(amount_outs)
    std_in = np.std(amount_ins)
    std_out = np.std(amount_outs)
    unique_traders = len(traders)

    period = (times[-1] - times[0]).days
    if period == 0:
        period = 1

    tx_per_day = number_of_trades / period
    return (
        mean_in,
        mean_out,
        std_in,
        std_out,
        unique_traders,
        tx_per_day,
        number_of_trades,
    )


def create_df(arbitrage_paths):
    results = []
    for path in arbitrage_paths:
        results.append(
            {
                "path": " -> ".join(path["path"]),
                "rates_product": path["product_of_rates"],
                "reserves": path["reserves"],
                "slippages": path["slippages"],
                "profit": calculate_profit_symbolic(path["reserves"], TON_AMOUNT),
            }
        )

    final_df = pd.DataFrame(results)
    return final_df


if __name__ == "__main__":
    arbitrage_paths = main()
    df = create_df(arbitrage_paths)
    df.to_csv("arbitrage_results.csv", index=False)
