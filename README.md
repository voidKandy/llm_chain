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
+ `tests/scripts/try_rpc.sh` - Uses the `rpc.rs` & `server/main.rs` binaries to test the JSON RPC API for the server node. This script depends on tmux 

