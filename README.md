# Experimental Model Inference Blockchain
This is an experiment I am undergoing to learn how to build with blockchain technology, namely the `libp2p` stack.

## Project Structure
### `core`
A library for building on this chain I am implementing. Not sure what to call it, but `core` is fine for now.
### `client`
An example of an implementation of a client node on this chain using methods, traits and structs from the `core` library.
### `server`
An example of an implementation of a server node on this chain using methods, traits and structs from the `core` library.
### `macros`
Helper derive macros for an assortment of traits from the `core` library. 
Currently empty

## Scripts
All scripts depend on tmux
+ `tests/scripts/try_rpc.sh` - Uses the `rpc.rs` & `server/main.rs` binaries to test the JSON RPC API for the server node
+ `tests/scripts/client_start_auction.sh` - Uses multiple binaries to do the following: 
    1. Spin up provider boot node
    2. Spin up client node & connect to network through boot node
    3. Send a JSON RPC request to the client to start auctioning 

