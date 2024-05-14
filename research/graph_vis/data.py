from pydantic import BaseModel
import requests as req
from typing import Any

TON_NATIVE_ADDRESS = "EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c"


class Token(BaseModel):
    native: bool
    address: str
    name: str | None
    symbol: str | None
    dex: list[str]


class Pool(BaseModel):
    address: str
    token0: Token
    token1: Token
    reserve0: int
    reserve1: int
    dex: str


def process_token_dedust(token: dict[str, Any]) -> Token:
    name = None
    symbol = None
    native = token.get("type") == "native"
    address = (
        TON_NATIVE_ADDRESS if native else token.get("address", "address not found")
    )

    metadata = token.get("metadata", {})
    if metadata:
        name = metadata.get("name")
        symbol = metadata.get("symbol")

    return Token(
        native=native, address=address, name=name, symbol=symbol, dex=["dedust"]
    )


def process_token_pair_dedust(data: list[dict[str, Any]]) -> tuple[Token, Token]:
    token0 = process_token_dedust(data[0])
    token1 = process_token_dedust(data[1])
    return token0, token1


def get_pools_dedust() -> list[Pool]:
    r = req.get("https://api.dedust.io/v2/pools")
    pools = r.json()
    pool_list: list[Pool] = []
    for pool in pools:
        token0, token1 = process_token_pair_dedust(pool["assets"])
        reserve0 = int(pool["reserves"][0])
        reserve1 = int(pool["reserves"][1])
        pool_list.append(
            Pool(
                address=pool["address"],
                token0=token0,
                token1=token1,
                reserve0=reserve0,
                reserve1=reserve1,
                dex="dedust",
            )
        )
    return pool_list


def unpack_dedust_tokens(pools: list[Pool]) -> dict[str, Token]:
    tokens: dict[str, Token] = {}
    for pool in pools:
        tokens[pool.token0.address] = pool.token0
        tokens[pool.token1.address] = pool.token1
    return tokens


def get_pools_stonfi(tokens: dict[str, Token]) -> tuple[list[Pool], dict[str, Token]]:
    r = req.get("https://api.ston.fi/v1/pools")
    pools = r.json()["pool_list"]

    stonfi_pools: list[Pool] = []
    for pool in pools:
        token0 = tokens.get(pool["token0_address"])
        if not token0:
            token0 = Token(
                native=False,
                address=pool["token0_address"],
                name=None,
                symbol=None,
                dex=["stonfi"],
            )
            tokens[token0.address] = token0
        elif token0 and "stonfi" not in token0.dex:
            tokens[token0.address].dex.append("stonfi")

        token1 = tokens.get(pool["token1_address"])
        if not token1:
            token1 = Token(
                native=False,
                address=pool["token1_address"],
                name=None,
                symbol=None,
                dex=["stonfi"],
            )
            tokens[token1.address] = token1
        elif token1 and "stonfi" not in token1.dex:
            tokens[token1.address].dex.append("stonfi")

        stonfi_pools.append(
            Pool(
                address=pool["address"],
                token0=token0,
                token1=token1,
                reserve0=int(pool["reserve0"]),
                reserve1=int(pool["reserve1"]),
                dex="stonfi",
            )
        )
    return stonfi_pools, tokens


def get_all_pools_and_tokens():
    dedust_pools = get_pools_dedust()
    dedust_tokens = unpack_dedust_tokens(dedust_pools)
    stonfi_pools, tokens = get_pools_stonfi(dedust_tokens)
    pools = dedust_pools + stonfi_pools
    return pools, tokens


def get_dedust_pools_and_tokens():
    dedust_pools = get_pools_dedust()
    dedust_tokens = unpack_dedust_tokens(dedust_pools)
    return dedust_pools, dedust_tokens


if __name__ == "__main__":
    pools, tokens = get_dedust_pools_and_tokens()
