import sys

sys.path.append(
    "/Users/sbr/dev/aceton/research/graph_vis"
)  # ignore this line if you are not running this in a notebook

from data import get_all_pools_and_tokens, get_dedust_pools_and_tokens
import networkx as nx
import pandas as pd

FEE = 0.003


def create_graph() -> nx.Graph:
    pools, tokens = get_dedust_pools_and_tokens()
    graph = nx.DiGraph()

    for token in tokens.values():

        graph.add_node(  # type: ignore
            token.address,
            name=token.name,
            symbol=token.symbol,
            dedust=True if "dedust" in token.dex else False,
            stonfi=True if "stonfi" in token.dex else False,
        )

    for pool in pools:
        graph.add_edge(  # type: ignore
            pool.token0.address,
            pool.token1.address,
            dex=pool.dex,
            address=pool.address,
            reserve0=pool.reserve0,
            reserve1=pool.reserve1,
            rate=pool.reserve0 / pool.reserve1 if pool.reserve1 != 0 else 0,
            fee=1 - FEE,
        ),
        graph.add_edge(  # type: ignore
            pool.token1.address,
            pool.token0.address,
            dex=pool.dex,
            address=pool.address,
            reserve0=pool.reserve0,
            reserve1=pool.reserve1,
            rate=pool.reserve0 / pool.reserve1 if pool.reserve1 != 0 else 0,
            fee=1 - FEE,
        )

    return graph


if __name__ == "__main__":
    graph = create_graph()
    df = pd.DataFrame(graph.nodes.data())
    df.columns = ["id", "data"]
    df = df[["id"]].merge(
        df["data"].apply(pd.Series), left_index=True, right_index=True
    )
    df.to_csv("graph_metadata.csv", index=False)
    nx.to_pandas_edgelist(graph).to_csv("graph_data.csv", index=False)
