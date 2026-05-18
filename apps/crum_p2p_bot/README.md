# crum_p2p_bot

`crum_p2p_bot` demonstrates a three-player peer-to-peer poker hand:

- table/lobby state is on the Private Poker Stylus contracts;
- WebRTC signalling is exchanged through `PrivatePokerSignal`;
- poker state updates move around the ring over WebRTC;
- each bot prints the same hand transcript.

Run these commands after Nitro DevNode is running, the contracts are deployed, and lobby `1` exists.

## Player 1 TTY

```sh
cargo run -p crum_p2p_bot -- \
  --rpc-url http://localhost:8547 \
  --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
  --config crum_p2p_bot.toml \
  --lobby 1 \
  --create-table texas_holdem.toml \
  --table-name "My 3P Game" \
  --num-players 3
```

## Player 2 TTY

```sh
cargo run -p crum_p2p_bot -- \
  --rpc-url http://localhost:8547 \
  --private-key 0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d \
  --config crum_p2p_bot.toml \
  --lobby 1 \
  --join-table \
  --table-name "My 3P Game"
```

## Player 3 TTY

```sh
cargo run -p crum_p2p_bot -- \
  --rpc-url http://localhost:8547 \
  --private-key 0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a \
  --config crum_p2p_bot.toml \
  --lobby 1 \
  --join-table \
  --table-name "My 3P Game"
```

Use the same `--table-name` in all three terminals. Start Player 1 first, then Player 2 and Player 3.
