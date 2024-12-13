#!/bin/bash

SESSION_NAME=$(tmux display-message -p '#S')
WINDOW_ID=$(tmux display-message -p '#I')
PANE_COUNT=$(tmux list-panes -t $SESSION_NAME:$WINDOW_ID | wc -l)

if [ "$PANE_COUNT" -eq 1 ]; then
    tmux split-window -h
    tmux select-pane -t $SESSION_NAME:$WINDOW_ID.1
    tmux split-window -v
fi

if [ "$PANE_COUNT" -eq 2 ]; then
    tmux select-pane -t $SESSION_NAME:$WINDOW_ID.1
    tmux split-window -v
fi

tmux send-keys -t $SESSION_NAME:$WINDOW_ID.1 C-c
tmux send-keys -t $SESSION_NAME:$WINDOW_ID.1 "clear" C-m

tmux send-keys -t $SESSION_NAME:$WINDOW_ID.2 C-c
tmux send-keys -t $SESSION_NAME:$WINDOW_ID.2 "clear" C-m

tmux send-keys -t $SESSION_NAME:$WINDOW_ID.1 'cargo run --bin main -- -s true | bunyan' C-m

sleep 1.8

LISTENING_ADDR=$(tmux capture-pane -t $SESSION_NAME:$WINDOW_ID.1 -pS -5 | sed -n "s|.*127\.0\.0\.1/udp/\([0-9][0-9]*\).*|\1|p" )



if [ -z "$LISTENING_ADDR" ]; then
    echo "Error: No listening address found in pane 1"
    exit 1
fi

echo "found port: $LISTENING_ADDR"

tmux send-keys -t $SESSION_NAME:$WINDOW_ID.2 "cargo run --bin main -- /ip4/127.0.0.1/udp/$LISTENING_ADDR/quic-v1 | bunyan" C-m
sleep 1.8

tmux send-keys -t $SESSION_NAME:$WINDOW_ID.2 "hello" C-m

# sleep.2
# tmux send-keys -t $SESSION_NAME:$WINDOW_ID.2 C-c
# tmux send-keys -t $SESSION_NAME:$WINDOW_ID.1 C-c

tmux select-pane -t $SESSION_NAME:$WINDOW_ID.0
